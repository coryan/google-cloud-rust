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
	"testing"

	"github.com/google/go-cmp/cmp"
	"github.com/googleapis/google-cloud-rust/generator/internal/api"
	"github.com/googleapis/google-cloud-rust/generator/internal/config"
)

func TestBuildRootOptions(t *testing.T) {
	model := api.NewTestAPI([]*api.Message{}, []*api.Enum{}, []*api.Service{})
	cfg := &config.Config{
		Codec: map[string]string{
			"copyright-year": "2034",
		},
	}
	got, err := buildRoot(model, cfg)
	if err != nil {
		t.Fatal(err)
	}
	if got.CopyrightYear != "2034" {
		t.Errorf("mismatched copyright year, want=%s, got=%s", "2034", got.CopyrightYear)
	}
}

func TestBuildRootPackages(t *testing.T) {
	message1 := &api.Message{
		Name:    "Target",
		ID:      ".test.v1.Target",
		Package: "test.v1",
	}
	message2 := &api.Message{
		Name:    "M2",
		ID:      ".test.v1.child.M2",
		Package: "test.v1.child",
	}
	anyz := &api.Message{
		Name:    "Any",
		ID:      ".google.protobuf.Any",
		Package: "google.protobuf",
	}
	enum := &api.Enum{
		Name:    "Enum",
		ID:      ".test.v2.Enum",
		Package: "test.v2",
	}
	service := &api.Service{
		Name:    "Service",
		ID:      ".test.type.Service",
		Package: "test.type",
	}
	model := api.NewTestAPI([]*api.Message{message1, message2, anyz}, []*api.Enum{enum}, []*api.Service{service})

	got, err := buildRoot(model, &config.Config{})
	if err != nil {
		t.Fatal(err)
	}

	want := []*protobufPackage{
		{
			Name:     "google",
			ID:       "google",
			Filename: "",
			Children: []*protobufPackage{
				{
					Name:     "protobuf",
					ID:       "google.protobuf",
					Filename: "",
				},
			},
		},
		{
			Name:     "test",
			ID:       "test",
			Filename: "",
			Children: []*protobufPackage{
				{
					Name:     "r#type",
					ID:       "test.type",
					Filename: "test.type.rs",
				},
				{
					Name:     "v1",
					ID:       "test.v1",
					Filename: "test.v1.rs",
					Children: []*protobufPackage{
						{Name: "child", ID: "test.v1.child", Filename: "test.v1.child.rs"},
					},
				},
				{
					Name:     "v2",
					ID:       "test.v2",
					Filename: "test.v2.rs",
				},
			},
		},
	}
	if diff := cmp.Diff(want, got.Packages); diff != "" {
		t.Errorf("mismatch in testBuildRoot (-want, +got)\n:%s", diff)
	}
}
