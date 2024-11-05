// Copyright 2024 Google LLC
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
	"flag"
	"fmt"
	"log"
	"log/slog"
	"os"
	"os/exec"
	"path/filepath"
)

var (
	language      = flag.String("language", "", "the generated language")
	output        = flag.String("out", "output", "the path to the output directory")
	protoFiles    = flag.String("files", "testdata/googleapis/google/cloud/secretmanager/v1/", "path to protos to generate from")
	protoPath     = flag.String("proto_path", "testdata/googleapis", "directory in which to search for imports")
	serviceConfig = flag.String("service-config", "testdata/google/cloud/secretmanager/v1/secretmanager_v1.yaml", "path to service config")
)

func main() {
	flag.Parse()
	if *language == "" {
		log.Fatalf("language must be provided")
	}
	if err := run(*language, *protoPath, *protoFiles, *output); err != nil {
		log.Fatal(err)
	}
}

func run(language, testdata, input, output string) error {
	var files []string
	err := filepath.Walk(input, func(path string, info os.FileInfo, err error) error {
		if err != nil {
			return err
		}
		if filepath.Ext(path) == ".proto" {
			files = append(files, path)
		}
		return nil
	})
	if err != nil {
		return err
	}

	args := []string{
		"-I", testdata,
		fmt.Sprintf("--gclient_out=%s", output),
		fmt.Sprintf("--gclient_opt=language=%s,service-config=%s", language, *serviceConfig),
	}
	args = append(args, files...)

	cmd := exec.Command("protoc", args...)
	slog.Info(cmd.String())

	cmd.Stdout = os.Stdout // or any other io.Writer
	cmd.Stderr = os.Stderr // or any other io.Writer
	return cmd.Run()
}