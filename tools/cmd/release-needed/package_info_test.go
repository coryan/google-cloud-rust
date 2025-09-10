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
)

func TestManifestInfoSuccess(t *testing.T) {
	dir := t.TempDir()
	manifest := path.Join(dir, "Cargo.toml")
	contents := fmt.Sprintf(initialCargoContents, "google-cloud-storage")
	if err := os.WriteFile(manifest, []byte(contents), 0644); err != nil {
		t.Fatal(err)
	}
	got, err := manifestInfo(manifest)
	if err != nil {
		t.Fatal(err)
	}
	want := &packageInfo{
		Name:    "google-cloud-storage",
		Version: "1.0.0",
		Publish: true,
	}
	if diff := cmp.Diff(want, got); diff != "" {
		t.Errorf("generated files changed mismatch (-want +got):\n%s", diff)
	}
}

func TestManifestInfoNoPublish(t *testing.T) {
	dir := t.TempDir()
	manifest := path.Join(dir, "Cargo.toml")
	contents := fmt.Sprintf(initialCargoContents, "google-cloud-storage")
	contents = contents + "\n" + "publish = false\n"
	if err := os.WriteFile(manifest, []byte(contents), 0644); err != nil {
		t.Fatal(err)
	}
	got, err := manifestInfo(manifest)
	if err != nil {
		t.Fatal(err)
	}
	want := &packageInfo{
		Name:    "google-cloud-storage",
		Version: "1.0.0",
		Publish: false,
	}
	if diff := cmp.Diff(want, got); diff != "" {
		t.Errorf("generated files changed mismatch (-want +got):\n%s", diff)
	}
}

func TestManifestInfoMissingFile(t *testing.T) {
	dir := t.TempDir()
	manifest := path.Join(dir, "Cargo.toml")
	if got, err := manifestInfo(manifest); err == nil {
		t.Errorf("expected an error with missing file, got=%v", got)
	}
}

func TestManifestInfoBadContents(t *testing.T) {
	dir := t.TempDir()
	manifest := path.Join(dir, "Cargo.toml")
	if err := os.WriteFile(manifest, []byte("hello world!"), 0644); err != nil {
		t.Fatal(err)
	}
	if got, err := manifestInfo(manifest); err == nil {
		t.Errorf("expected an error with bad manifest content, got=%v", got)
	}
}
