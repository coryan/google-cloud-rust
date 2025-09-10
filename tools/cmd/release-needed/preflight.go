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

	"github.com/googleapis/google-cloud-rust/tools/internal/preflight"
)

func preflightGit(gitExe string) error {
	return preflight.TestExternalCommand(gitExe, "--version")
}

func runPreflight(config *releaseConfig) error {
	if err := preflightGit(config.GitExe); err != nil {
		return err
	}
	if err := preflight.TestExternalCommand(config.GitExe, "remote", "get-url", "upstream"); err != nil {
		return fmt.Errorf("required git remote `upstream`, create the remote using `git remote add upstream https://github.com/googleapis/google-cloud-rust`: %w", err)
	}
	if err := preflight.TestExternalCommand(config.CargoExe, "--version"); err != nil {
		return fmt.Errorf("got an error trying to run `cargo --version`, the instructions on https://www.rust-lang.org/learn/get-started may solve this problem: %w", err)
	}
	return nil
}
