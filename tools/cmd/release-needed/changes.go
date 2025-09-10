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
	"path"
	"slices"
	"strings"
)

func filesChangedSince(config *releaseConfig, ref string) ([]string, error) {
	diff, err := cmdOutput(config.GitExe, "diff", "--name-only", ref)
	if err != nil {
		return nil, err
	}
	files := strings.Split(string(diff), "\n")
	files = slices.DeleteFunc(files, func(a string) bool {
		name := path.Base(a)
		if name == "." || config.SkippedFiles[name] == true {
			return true
		}
		return false
	})
	return files, nil
}
