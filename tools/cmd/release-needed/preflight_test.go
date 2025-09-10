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
	"testing"

	"github.com/googleapis/google-cloud-rust/tools/internal/preflight"
)

const (
	repoUrl = "https://github.com/googleapis/google-cloud-rust"
)

func TestHasGit(t *testing.T) {
	// The tests need git to be present.
	if err := preflightGit("git"); err != nil {
		t.Fatal(err)
	}
}

func TestPreflightNoGit(t *testing.T) {
	config := &releaseConfig{
		GitExe:   "git-not-found",
		CargoExe: "cargo-not-found",
	}
	if err := runPreflight(config); err == nil {
		t.Errorf("expected an error with bad git and cargo command names")
	}
}

func TestPreflightNoUpstream(t *testing.T) {
	config := &releaseConfig{
		GitExe:   "git",
		CargoExe: "cargo-not-found",
	}
	if err := preflightGit(config.GitExe); err != nil {
		t.Fatal(err)
	}

	outDir := t.TempDir()
	if err := initRepository(t, outDir); err != nil {
		t.Fatal(err)
	}
	if err := runPreflight(config); err == nil {
		t.Errorf("expected an error with missing upstream directory")
	}
}

func TestPreflightNoCargo(t *testing.T) {
	config := &releaseConfig{
		GitExe:   "git",
		CargoExe: "cargo-not-found",
	}
	if err := preflightGit(config.GitExe); err != nil {
		t.Fatal(err)
	}

	outDir := t.TempDir()
	if err := initRepository(t, outDir); err != nil {
		t.Fatal(err)
	}
	if err := preflight.TestExternalCommand(config.GitExe, "remote", "add", "upstream", repoUrl); err != nil {
		t.Fatal(err)
	}
	if err := runPreflight(config); err == nil {
		t.Errorf("expected an error when cargo program is missing")
	}
}

func TestPreflight(t *testing.T) {
	config := &releaseConfig{
		GitExe:   "git",
		CargoExe: "cargo",
	}
	if err := preflightGit("git"); err != nil {
		t.Fatal(err)
	}

	outDir := t.TempDir()
	if err := initRepository(t, outDir); err != nil {
		t.Fatal(err)
	}
	if err := preflight.TestExternalCommand(config.GitExe, "remote", "add", "upstream", repoUrl); err != nil {
		t.Fatal(err)
	}
	if err := runPreflight(config); err != nil {
		t.Fatal(err)
	}
}

func initRepository(t *testing.T, outDir string) error {
	t.Helper()
	if err := os.Chdir(outDir); err != nil {
		t.Fatal(err)
	}
	if err := preflight.TestExternalCommand("git", "init"); err != nil {
		t.Fatal(err)
	}
	return nil
}
