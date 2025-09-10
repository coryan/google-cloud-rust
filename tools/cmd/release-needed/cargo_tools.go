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

func installCargoTools(config *releaseConfig) error {
	tools := []string{
		fmt.Sprintf("release-plz@%s", config.ReleasePlzVersion),
		fmt.Sprintf("cargo-workspaces@%s", config.WorkspacesVersion),
		fmt.Sprintf("cargo-semver-checks@%s", config.SemverChecksVersion),
	}

	for _, target := range tools {
		if err := preflight.TestExternalCommand(config.CargoExe, "install", "--locked", target); err != nil {
			return err
		}
	}
	return nil
}
