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
	"os"
	"path"
	"testing"

	"github.com/google/go-cmp/cmp"
	"github.com/googleapis/google-cloud-rust/tools/internal/preflight"
)

func TestRunNoChanges(t *testing.T) {
	initClonedRepo(t)
	got, err := run(defaultConfig())
	if err != nil {
		t.Fatal(err)
	}
	var want []string
	if diff := cmp.Diff(want, got); diff != "" {
		t.Errorf("generated package list mismatch (-want +got):\n%s", diff)
	}
	// This just improves the code coverage.
	main()
}

func TestRunWithChanges(t *testing.T) {
	initClonedRepo(t)
	changedFile := "src/storage/src/lib.rs"
	if err := os.WriteFile(changedFile, []byte(changedLibRsContents), 0644); err != nil {
		t.Fatal(err)
	}
	if err := preflight.TestExternalCommand("git", "commit", "-m", "changed", "."); err != nil {
		t.Fatal(err)
	}
	got, err := run(defaultConfig())
	if err != nil {
		t.Fatal(err)
	}
	want := []string{"google-cloud-storage"}
	if diff := cmp.Diff(want, got); diff != "" {
		t.Errorf("generated package list mismatch (-want +got):\n%s", diff)
	}
}

func TestRunTagError(t *testing.T) {
	upstreamDir := t.TempDir()
	if err := initRepository(t, upstreamDir); err != nil {
		t.Fatal(err)
	}
	repoDir := t.TempDir()
	os.Chdir(repoDir)
	if err := preflight.TestExternalCommand("git", "clone", path.Join(upstreamDir, ".git"), "."); err != nil {
		t.Fatal(err)
	}
	if err := preflight.TestExternalCommand("git", "remote", "add", "upstream", path.Join(upstreamDir, ".git")); err != nil {
		t.Fatal(err)
	}
	if err := preflight.TestExternalCommand("git", "fetch", "upstream"); err != nil {
		t.Fatal(err)
	}
	if got, err := run(defaultConfig()); err == nil {
		t.Errorf("expected an error when tag is missing, got=%v", got)
	}
}

func initClonedRepo(t *testing.T) {
	t.Helper()
	upstreamDir := t.TempDir()
	if err := initRepository(t, upstreamDir); err != nil {
		t.Fatal(err)
	}
	if err := addHistoryToRepository(t); err != nil {
		t.Fatal(err)
	}
	repoDir := t.TempDir()
	os.Chdir(repoDir)
	if err := preflight.TestExternalCommand("git", "clone", path.Join(upstreamDir, ".git"), "."); err != nil {
		t.Fatal(err)
	}
	if err := preflight.TestExternalCommand("git", "remote", "add", "upstream", path.Join(upstreamDir, ".git")); err != nil {
		t.Fatal(err)
	}
	if err := preflight.TestExternalCommand("git", "fetch", "upstream"); err != nil {
		t.Fatal(err)
	}
}
