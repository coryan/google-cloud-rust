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
	"errors"
	"fmt"
	"log"
	"log/slog"
	"os/exec"
	"strings"
)

func main() {
	config := defaultConfig()
	packages, err := run(config)
	if err != nil {
		log.Fatal(err)
	}
	for _, p := range packages {
		fmt.Printf("* `%s`\n", p)
	}
}

func run(config *releaseConfig) ([]string, error) {
	slog.Info("verifying Rust release bootstrapping tools are installed")
	if err := runPreflight(config); err != nil {
		return nil, err
	}
	slog.Info("installing tools for Rust release")
	if err := installCargoTools(config); err != nil {
		return nil, err
	}
	slog.Info("detecting last release tag")
	lastTag, err := cmdOutput(config.GitExe, "describe", "--abbrev=0", "--tags", "upstream/main")
	if err != nil {
		return nil, err
	}
	tag := strings.TrimSuffix(string(lastTag), "\n")
	slog.Info("found last release tag", "tag", tag)
	files, err := filesChangedSince(config, tag)
	if err != nil {
		return nil, err
	}
	slog.Info("number of files changed since lat tag", "count", len(files))
	manifests, err := findCargoManifests(files)
	if err != nil {
		return nil, err
	}
	slog.Info("number of manifest files affected", "count", len(manifests))
	var packages []string
	for _, m := range manifests {
		info, err := manifestInfo(m)
		if err != nil {
			return nil, err
		}
		if info.Publish {
			packages = append(packages, info.Name)
		}
	}
	slog.Info("packages affected", "names", packages)
	var errs []error
	for _, p := range packages {
		slog.Info("updating version", "package", p)
		cmd := exec.Command("release-plz", "update", "--no-changelog", "--allow-dirty", "-p", p)
		cmd.Dir = "."
		if err := cmd.Run(); err != nil {
			errs = append(errs, err)
		}
	}
	if len(errs) > 0 {
		return nil, fmt.Errorf("errors updating packages: %w", errors.Join(errs...))
	}

	return packages, nil
}

func cmdOutput(command string, arg ...string) ([]byte, error) {
	cmd := exec.Command(command, arg...)
	cmd.Dir = "."
	return cmd.CombinedOutput()
}
