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

package language

import (
	"fmt"
	"strings"

	"github.com/googleapis/google-cloud-rust/generator/internal/api"
	"github.com/googleapis/google-cloud-rust/generator/internal/license"
	"github.com/iancoleman/strcase"
)

type GoTemplateData struct {
	Name              string
	Title             string
	Description       string
	PackageName       string
	SourcePackageName string
	HasServices       bool
	CopyrightYear     string
	BoilerPlate       []string
	Imports           []string
	DefaultHost       string
	Services          []*GoService
	Messages          []*GoMessage
	Enums             []*GoEnum
	GoPackage         string
}

type GoService struct {
	Methods             []*GoMethod
	NameToPascal        string
	ServiceNameToPascal string
	NameToCamel         string
	ServiceName         string
	DocLines            []string
	DefaultHost         string
}

type GoMessage struct {
	Fields             []*GoField
	BasicFields        []*GoField
	ExplicitOneOfs     []*GoOneOf
	NestedMessages     []*GoMessage
	Enums              []*GoEnum
	Name               string
	QualifiedName      string
	HasNestedTypes     bool
	DocLines           []string
	IsMap              bool
	IsPageableResponse bool
	PageableItem       *GoField
	ID                 string
	// The FQN is the source specification
	SourceFQN string
}

type GoMethod struct {
	NameToCamel         string
	NameToPascal        string
	DocLines            []string
	InputTypeName       string
	OutputTypeName      string
	HTTPMethod          string
	HTTPMethodToLower   string
	HTTPPathFmt         string
	HTTPPathArgs        []string
	PathParams          []*GoField
	QueryParams         []*GoField
	HasBody             bool
	BodyAccessor        string
	IsPageable          bool
	ServiceNameToPascal string
	ServiceNameToCamel  string
	InputTypeID         string
	InputType           *GoMessage
	OperationInfo       *GoOperationInfo
}

type GoOperationInfo struct {
	MetadataType string
	ResponseType string
}

type GoOneOf struct {
	NameToPascal string
	DocLines     []string
	Fields       []*GoField
}

type GoField struct {
	NameToCamel        string
	NameToPascal       string
	DocLines           []string
	FieldType          string
	PrimitiveFieldType string
	JSONName           string
	AsQueryParameter   string
}

type GoEnum struct {
	Name     string
	DocLines []string
	Values   []*GoEnumValue
}

type GoEnumValue struct {
	DocLines []string
	Name     string
	Number   int32
	EnumType string
}

// newGoTemplateData creates a struct used as input for Mustache templates.
// Fields and methods defined in this struct directly correspond to Mustache
// tags. For example, the Mustache tag {{#Services}} uses the
// [Template.Services] field.
func newGoTemplateData(model *api.API, options map[string]string) (*GoTemplateData, error) {
	var (
		sourceSpecificationPackageName string
		packageNameOverride            string
		packageName                    string
		generationYear                 string
		importMap                      = map[string]*goImport{}
	)

	for key, definition := range options {
		switch {
		case key == "package-name-override":
			packageNameOverride = definition
		case key == "go-package-name":
			packageName = definition
		case key == "copyright-year":
			generationYear = definition
		case strings.HasPrefix(key, "import-mapping"):
			keys := strings.Split(key, ":")
			if len(keys) != 2 {
				return nil, fmt.Errorf("key should be in the format import-mapping:proto.path, got=%q", key)
			}
			defs := strings.Split(definition, ";")
			if len(defs) != 2 {
				return nil, fmt.Errorf("%s should be in the format path;name, got=%q", definition, keys[1])
			}
			importMap[keys[1]] = &goImport{
				path: defs[0],
				name: defs[1],
			}
		}
	}
	goValidate(model, sourceSpecificationPackageName)

	goLoadWellKnownTypes(model.State)
	data := &GoTemplateData{
		Name:              model.Name,
		Title:             model.Title,
		Description:       model.Description,
		PackageName:       goPackageName(model, packageNameOverride),
		SourcePackageName: sourceSpecificationPackageName,
		HasServices:       len(model.Services) > 0,
		CopyrightYear:     generationYear,
		BoilerPlate: append(license.LicenseHeaderBulk(),
			"",
			" Code generated by sidekick. DO NOT EDIT."),
		Imports: goImports(importMap),
		DefaultHost: func() string {
			if len(model.Services) > 0 {
				return model.Services[0].DefaultHost
			}
			return ""
		}(),
		Services: mapSlice(model.Services, func(s *api.Service) *GoService {
			return newGoService(s, model.State, importMap)
		}),
		Messages: mapSlice(model.Messages, func(m *api.Message) *GoMessage {
			return newGoMessage(m, model.State, importMap)
		}),
		Enums: mapSlice(model.Enums, func(e *api.Enum) *GoEnum {
			return newGoEnum(e, model.State, importMap)
		}),
		GoPackage: packageName,
	}

	messagesByID := map[string]*GoMessage{}
	for _, m := range data.Messages {
		messagesByID[m.ID] = m
	}
	for _, s := range data.Services {
		for _, method := range s.Methods {
			if msg, ok := messagesByID[method.InputTypeID]; ok {
				method.InputType = msg
			} else if m, ok := model.State.MessageByID[method.InputTypeID]; ok {
				method.InputType = newGoMessage(m, model.State, importMap)
			}
		}
	}
	return data, nil
}

func newGoService(s *api.Service, state *api.APIState, importMap map[string]*goImport) *GoService {
	// Some codecs skip some methods.
	methods := filterSlice(s.Methods, func(m *api.Method) bool {
		return goGenerateMethod(m)
	})
	return &GoService{
		Methods: mapSlice(methods, func(m *api.Method) *GoMethod {
			return newGoMethod(m, state, importMap)
		}),
		NameToPascal:        goToPascal(s.Name),
		ServiceNameToPascal: goToPascal(s.Name), // Alias for clarity
		NameToCamel:         strcase.ToLowerCamel(s.Name),
		ServiceName:         s.Name,
		DocLines:            goFormatDocComments(s.Documentation, state),
		DefaultHost:         s.DefaultHost,
	}
}

func newGoMessage(m *api.Message, state *api.APIState, importMap map[string]*goImport) *GoMessage {
	return &GoMessage{
		Fields: mapSlice(m.Fields, func(s *api.Field) *GoField {
			return newGoField(s, state, importMap)
		}),
		BasicFields: func() []*GoField {
			filtered := filterSlice(m.Fields, func(s *api.Field) bool {
				return !s.IsOneOf
			})
			return mapSlice(filtered, func(s *api.Field) *GoField {
				return newGoField(s, state, importMap)
			})
		}(),
		ExplicitOneOfs: mapSlice(m.OneOfs, func(s *api.OneOf) *GoOneOf {
			return newGoOneOf(s, state, importMap)
		}),
		NestedMessages: mapSlice(m.Messages, func(s *api.Message) *GoMessage {
			return newGoMessage(s, state, importMap)
		}),
		Enums: mapSlice(m.Enums, func(s *api.Enum) *GoEnum {
			return newGoEnum(s, state, importMap)
		}),
		Name:          goMessageName(m, importMap),
		QualifiedName: goMessageName(m, importMap),
		HasNestedTypes: func() bool {
			if len(m.Enums) > 0 || len(m.OneOfs) > 0 {
				return true
			}
			for _, child := range m.Messages {
				if !child.IsMap {
					return true
				}
			}
			return false
		}(),
		DocLines:           goFormatDocComments(m.Documentation, state),
		IsMap:              m.IsMap,
		IsPageableResponse: m.IsPageableResponse,
		PageableItem:       newGoField(m.PageableItem, state, importMap),
		ID:                 m.ID,
		SourceFQN:          strings.TrimPrefix(m.ID, "."),
	}
}

func newGoMethod(m *api.Method, state *api.APIState, importMap map[string]*goImport) *GoMethod {
	method := &GoMethod{
		BodyAccessor:      goBodyAccessor(m),
		DocLines:          goFormatDocComments(m.Documentation, state),
		HTTPMethod:        m.PathInfo.Verb,
		HTTPMethodToLower: strings.ToLower(m.PathInfo.Verb),
		HTTPPathArgs:      goHTTPPathArgs(m.PathInfo),
		HTTPPathFmt:       goHTTPPathFmt(m.PathInfo),
		HasBody:           m.PathInfo.BodyFieldPath != "",
		InputTypeName:     goMethodInOutTypeName(m.InputTypeID, state),
		NameToCamel:       strcase.ToCamel(m.Name),
		NameToPascal:      goToPascal(m.Name),
		OutputTypeName:    goMethodInOutTypeName(m.OutputTypeID, state),
		PathParams: mapSlice(PathParams(m, state), func(s *api.Field) *GoField {
			return newGoField(s, state, importMap)
		}),
		QueryParams: mapSlice(QueryParams(m, state), func(s *api.Field) *GoField {
			return newGoField(s, state, importMap)
		}),
		IsPageable:          m.IsPageable,
		ServiceNameToPascal: goToPascal(m.Parent.Name),
		InputTypeID:         m.InputTypeID,
	}
	if m.OperationInfo != nil {
		method.OperationInfo = &GoOperationInfo{
			MetadataType: goMethodInOutTypeName(m.OperationInfo.MetadataTypeID, state),
			ResponseType: goMethodInOutTypeName(m.OperationInfo.ResponseTypeID, state),
		}
	}
	return method
}

func newGoOneOf(oneOf *api.OneOf, state *api.APIState, importMap map[string]*goImport) *GoOneOf {
	return &GoOneOf{
		NameToPascal: goToPascal(oneOf.Name),
		DocLines:     goFormatDocComments(oneOf.Documentation, state),
		Fields: mapSlice(oneOf.Fields, func(field *api.Field) *GoField {
			return newGoField(field, state, importMap)
		}),
	}
}

func newGoField(field *api.Field, state *api.APIState, importMap map[string]*goImport) *GoField {
	if field == nil {
		return nil
	}
	return &GoField{
		NameToCamel:        strcase.ToLowerCamel(field.Name),
		NameToPascal:       goToPascal(field.Name),
		DocLines:           goFormatDocComments(field.Documentation, state),
		FieldType:          goFieldType(field, state, importMap),
		PrimitiveFieldType: goFieldType(field, state, importMap),
		JSONName:           field.JSONName,
		AsQueryParameter:   goAsQueryParameter(field),
	}
}

func newGoEnum(e *api.Enum, state *api.APIState, importMap map[string]*goImport) *GoEnum {
	return &GoEnum{
		Name:     goEnumName(e, importMap),
		DocLines: goFormatDocComments(e.Documentation, state),
		Values: mapSlice(e.Values, func(s *api.EnumValue) *GoEnumValue {
			return newGoEnumValue(s, e, state, importMap)
		}),
	}
}

func newGoEnumValue(ev *api.EnumValue, e *api.Enum, state *api.APIState, importMap map[string]*goImport) *GoEnumValue {
	return &GoEnumValue{
		DocLines: goFormatDocComments(ev.Documentation, state),
		Name:     goEnumValueName(ev, importMap),
		Number:   ev.Number,
		EnumType: goEnumName(e, importMap),
	}
}