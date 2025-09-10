// Copyright 2025 Google LLC
//
// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License at
//
//     https://www.apache.org/licenses/LICENSE-2.0
//
// Unless required by applicable law or agreed to in writing, software
// distributed under the License is distributed on an "AS IS" BASIS,
// WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
// See the License for the specific language governing permissions and
// limitations under the License.

package main

import (
	"fmt"
	"os"
	"path"
	"testing"

	"github.com/google/go-cmp/cmp"
	"github.com/googleapis/google-cloud-rust/tools/internal/preflight"
)

const (
	initialCargoContents = `# Example Cargo file
[package]
name    = "%s"
version = "1.0.0"
`

	initialLibRsContents = `pub fn test() -> &'static str { "Hello World" }`

	changedLibRsContents = `pub fn hello() -> &'static str { "Hello World" }`
)

func TestChangedFiles(t *testing.T) {
	tmpDir := t.TempDir()
	if err := initRepository(t, tmpDir); err != nil {
		t.Fatal(err)
	}
	if err := addHistoryToRepository(t); err != nil {
		t.Fatal(err)
	}
	changedFile := "src/storage/src/lib.rs"
	if err := os.WriteFile(changedFile, []byte(changedLibRsContents), 0644); err != nil {
		t.Fatal(err)
	}
	if err := os.WriteFile("src/storage/.sidekick.toml", []byte("# newer version"), 0644); err != nil {
		t.Fatal(err)
	}
	if err := preflight.TestExternalCommand("git", "commit", "-m", "changed", "."); err != nil {
		t.Fatal(err)
	}
	config := defaultConfig()
	got, err := filesChangedSince(config, "v1.0.0")
	if err != nil {
		t.Fatal(err)
	}
	want := []string{changedFile}
	if diff := cmp.Diff(want, got); diff != "" {
		t.Errorf("generated files changed mismatch (-want +got):\n%s", diff)
	}
}

func TestChangedFilesBadRef(t *testing.T) {
	tmpDir := t.TempDir()
	if err := initRepository(t, tmpDir); err != nil {
		t.Fatal(err)
	}
	config := defaultConfig()
	if got, err := filesChangedSince(config, "v1.0.0"); err == nil {
		t.Errorf("expected an error with a bad ref, got=%v", got)
	}
}

func addCrate(t *testing.T, location, name string) error {
	t.Helper()
	_ = os.MkdirAll(path.Join(location, "src"), 0755)
	contents := []byte(fmt.Sprintf(initialCargoContents, name))
	if err := os.WriteFile(path.Join(location, "Cargo.toml"), contents, 0644); err != nil {
		t.Error(err)
		return err
	}
	if err := os.WriteFile(path.Join(location, "src", "lib.rs"), []byte(initialLibRsContents), 0644); err != nil {
		t.Error(err)
		return err
	}
	if err := os.WriteFile(path.Join(location, ".sidekick.toml"), []byte("# initial version"), 0644); err != nil {
		t.Error(err)
		return err
	}
	if err := os.WriteFile(path.Join(location, ".repo-metadata.json"), []byte("{}"), 0644); err != nil {
		t.Error(err)
		return err
	}
	return nil
}

func addHistoryToRepository(t *testing.T) error {
	t.Helper()
	if err := os.WriteFile("README.md", []byte("initial"), 0644); err != nil {
		t.Error(err)
		return err
	}
	if err := addCrate(t, "src/generated/cloud/secretmanager/v1", "google-cloud-secretmanager-v1"); err != nil {
		return err
	}
	if err := addCrate(t, "src/storage", "google-cloud-storage"); err != nil {
		return err
	}
	if err := preflight.TestExternalCommand("git", "add", "."); err != nil {
		t.Error(err)
		return err
	}
	if err := preflight.TestExternalCommand("git", "commit", "-m", "initial revision", "."); err != nil {
		t.Error(err)
		return err
	}
	if err := preflight.TestExternalCommand("git", "tag", "v1.0.0"); err != nil {
		return err
	}
	return nil
}
