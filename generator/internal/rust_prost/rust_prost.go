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

package rust_prost

import (
	"errors"
	"fmt"
	"log/slog"
	"maps"
	"os/exec"
	"path"

	"github.com/googleapis/google-cloud-rust/generator/internal/api"
	"github.com/googleapis/google-cloud-rust/generator/internal/config"
	"github.com/googleapis/google-cloud-rust/generator/internal/protobuf"
	"github.com/googleapis/google-cloud-rust/generator/internal/rust"
)

func Generate(model *api.API, outdir string, cfg *config.Config) error {
	if cfg.General.SpecificationFormat != "protobuf" {
		return fmt.Errorf("The `rust+prost` generator only supports `protobuf` as an specification source, outdir=%s", outdir)
	}
	if err := runExternalCommand("cargo", "--version"); err != nil {
		return fmt.Errorf("got an error trying to run `cargo --version`, the instructions on https://www.rust-lang.org/learn/get-started may solve this problem: %w", err)
	}
	if err := runExternalCommand("protoc", "--version"); err != nil {
		return fmt.Errorf("got an error trying to run `protoc --version`, the instructions on https://grpc.io/docs/protoc-installation/ may solve this problem: %w", err)
	}
	if err := runExternalCommand("protoc-gen-tonic", "--version"); err != nil {
		return fmt.Errorf("got an error trying to run `protoc-gen-tonic --version`, `cargo install protoc-gen-tonic` may solve this problem: %w", err)
	}
	if err := runExternalCommand("protoc-gen-prost", "--version"); err != nil {
		return fmt.Errorf("got an error trying to run `protoc-gen-prost --version`, `cargo install protoc-gen-prost` may solve this problem: %w", err)
	}
	codecOptions := maps.Clone(cfg.Codec)
	codecOptions["template-override"] = "templates/prost"
	if err := rust.Generate(model, outdir, codecOptions); err != nil {
		return err
	}

	if err := protoc(outdir, cfg); err != nil {
		return err
	}
	return nil
}

func protoc(outdir string, cfg *config.Config) error {
	files, err := protobuf.DetermineInputFiles(cfg.General.SpecificationSource, cfg.Source)
	if err != nil {
		return err
	}
	args := []string{
		"--prost_out", path.Join(outdir, "protos"),
		"--tonic_out", path.Join(outdir, "protos"),
	}
	for _, name := range config.SourceRoots(cfg.Source) {
		if path, ok := cfg.Source[name]; ok {
			args = append(args, "--proto_path")
			args = append(args, path)
		}
	}
	args = append(args, files...)
	return runExternalCommand("protoc", args...)
}

func runExternalCommand(c string, arg ...string) error {
	slog.Info("running protoc", "arg", arg)
	cmd := exec.Command(c, arg...)
	cmd.Dir = "."
	if output, err := cmd.CombinedOutput(); err != nil {
		if ee := (*exec.ExitError)(nil); errors.As(err, &ee) && len(ee.Stderr) > 0 {
			return fmt.Errorf("%v: %v\n%s", cmd, err, ee.Stderr)
		}
		return fmt.Errorf("%v: %v\n%s", cmd, err, output)
	}
	return nil
}
