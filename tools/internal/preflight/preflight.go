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

package preflight

import (
	"fmt"
	"os/exec"
)

// TestExternalCommand verifies that a give command can be executed with some
// arguments.
//
// This is used to check the development environment before running a complex
// pipeline, providing better error messages compared to a failure in the
// middle of the pipeline.
func TestExternalCommand(command string, arg ...string) error {
	cmd := exec.Command(command, arg...)
	cmd.Dir = "."
	if output, err := cmd.CombinedOutput(); err != nil {
		return fmt.Errorf("%v: %v\n%s", cmd, err, output)
	}
	return nil
}
