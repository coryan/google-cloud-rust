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

package parser

import (
	"bytes"
	"fmt"
	"log/slog"
	"maps"
	"os"
	"os/exec"
	"slices"
	"strings"

	"github.com/googleapis/google-cloud-rust/generator/internal/api"
	"github.com/googleapis/google-cloud-rust/generator/internal/config"
	"github.com/googleapis/google-cloud-rust/generator/internal/protobuf"
	"google.golang.org/genproto/googleapis/api/serviceconfig"
	"google.golang.org/protobuf/proto"
	"google.golang.org/protobuf/types/descriptorpb"
	"google.golang.org/protobuf/types/pluginpb"
)

// ParserProtobuf reads Protobuf specifications and converts them into
// the `api.API` model.
func ParseProtobuf(source, serviceConfigFile string, options map[string]string) (*api.API, error) {
	request, err := newCodeGeneratorRequest(source, options)
	if err != nil {
		return nil, err
	}
	var serviceConfig *serviceconfig.Service
	if serviceConfigFile != "" {
		cfg, err := readServiceConfig(findServiceConfigPath(serviceConfigFile, options))
		if err != nil {
			return nil, err
		}
		serviceConfig = cfg
	}
	return makeAPIForProtobuf(serviceConfig, request), nil
}

func newCodeGeneratorRequest(source string, options map[string]string) (_ *pluginpb.CodeGeneratorRequest, err error) {
	// Create a temporary files to store `protoc`'s output
	tempFile, err := os.CreateTemp("", "protoc-out-")
	if err != nil {
		return nil, err
	}
	defer func() {
		rerr := os.Remove(tempFile.Name())
		if err == nil {
			err = rerr
		}
	}()

	files, err := protobuf.DetermineInputFiles(source, options)
	if err != nil {
		return nil, err
	}

	// Call protoc with the given arguments.
	contents, err := protoc(tempFile.Name(), files, options)
	if err != nil {
		return nil, err
	}

	descriptors := &descriptorpb.FileDescriptorSet{}
	if err := proto.Unmarshal(contents, descriptors); err != nil {
		return nil, err
	}
	var target []*descriptorpb.FileDescriptorProto
	// Find all the file descriptors that correspond to the input files
	for _, filename := range files {
		for _, pb := range descriptors.File {
			// protoc requires files to be in a subdirectory of the
			// --proto_path options and it strips the option value from the
			// filename.
			if strings.HasSuffix(filename, *pb.Name) {
				target = append(target, pb)
			}
		}
	}
	request := &pluginpb.CodeGeneratorRequest{
		FileToGenerate:        files,
		SourceFileDescriptors: target,
		ProtoFile:             descriptors.File,
		CompilerVersion:       newCompilerVersion(),
	}
	return request, nil
}

func protoc(tempFile string, files []string, options map[string]string) ([]byte, error) {
	args := []string{
		"--include_imports",
		"--include_source_info",
		"--retain_options",
		"--descriptor_set_out", tempFile,
	}
	for _, name := range config.SourceRoots(options) {
		if path, ok := options[name]; ok {
			args = append(args, "--proto_path")
			args = append(args, path)
		}
	}

	args = append(args, files...)

	var stderr, stdout bytes.Buffer
	cmd := exec.Command("protoc", args...)
	cmd.Stderr = &stderr
	cmd.Stdout = &stdout
	if err := cmd.Run(); err != nil {
		return nil, fmt.Errorf("error calling protoc\ndetails:\n%s\nargs:\n%v\n: %w", stderr.String(), args, err)
	}

	return os.ReadFile(tempFile)
}

func newCompilerVersion() *pluginpb.Version {
	var (
		i int32
		s = "test"
	)
	return &pluginpb.Version{
		Major:  &i,
		Minor:  &i,
		Patch:  &i,
		Suffix: &s,
	}
}

const (
	// From https://pkg.go.dev/google.golang.org/protobuf/types/descriptorpb#FileDescriptorProto
	fileDescriptorName             = 1
	fileDescriptorPackage          = 2
	fileDescriptorDependency       = 3
	fileDescriptorMessageType      = 4
	fileDescriptorEnumType         = 5
	fileDescriptorService          = 6
	fileDescriptorExtension        = 7
	fileDescriptorOptions          = 8
	fileDescriptorSourceCodeInfo   = 9
	fileDescriptorPublicDependency = 10
	fileDescriptorWeakDependency   = 11
	fileDescriptorSyntax           = 12
	fileDescriptorEdition          = 14

	// From https://pkg.go.dev/google.golang.org/protobuf/types/descriptorpb#ServiceDescriptorProto
	serviceDescriptorProtoMethod = 2
	serviceDescriptorProtoOption = 3

	// From https://pkg.go.dev/google.golang.org/protobuf/types/descriptorpb#DescriptorProto
	messageDescriptorField          = 2
	messageDescriptorNestedType     = 3
	messageDescriptorEnum           = 4
	messageDescriptorExtensionRange = 5
	messageDescriptorExtension      = 6
	messageDescriptorOptions        = 7
	messageDescriptorOneOf          = 8

	// From https://pkg.go.dev/google.golang.org/protobuf/types/descriptorpb#EnumDescriptorProto
	enumDescriptorValue = 2
)

func makeAPIForProtobuf(serviceConfig *serviceconfig.Service, req *pluginpb.CodeGeneratorRequest) *api.API {
	var (
		mixinFileDesc       []*descriptorpb.FileDescriptorProto
		enabledMixinMethods mixinMethods = make(map[string]bool)
	)
	state := &api.APIState{
		ServiceByID: make(map[string]*api.Service),
		MethodByID:  make(map[string]*api.Method),
		MessageByID: make(map[string]*api.Message),
		EnumByID:    make(map[string]*api.Enum),
	}
	result := &api.API{
		State: state,
	}
	if serviceConfig != nil {
		result.Title = serviceConfig.Title
		if serviceConfig.Documentation != nil {
			result.Description = serviceConfig.Documentation.Summary
		}
		withLongrunning := requiresLongrunningMixin(req)
		enabledMixinMethods, mixinFileDesc = loadMixins(serviceConfig, withLongrunning)
		packageName := ""
		for _, api := range serviceConfig.Apis {
			packageName, _ = splitApiName(api.Name)
			// Keep searching after well-known mixin services.
			if !wellKnownMixin(api.Name) {
				break
			}
		}
		result.PackageName = packageName
	}

	// First we need to add all the message and enums types to the
	// `state.MessageByID` and `state.EnumByID` symbol tables. We may not need
	// to generate these elements, but we need them to be available to generate
	// any RPC that uses them.
	for _, f := range append(req.GetProtoFile(), mixinFileDesc...) {
		fFQN := "." + f.GetPackage()
		for _, m := range f.MessageType {
			mFQN := fFQN + "." + m.GetName()
			_ = processMessage(state, m, mFQN, f.GetPackage(), nil)
		}

		for _, e := range f.EnumType {
			eFQN := fFQN + "." + e.GetName()
			_ = processEnum(state, e, eFQN, f.GetPackage(), nil)
		}
	}

	// Then we need to add the messages, enums and services to the list of
	// elements to be generated.
	for _, f := range req.GetSourceFileDescriptors() {
		var fileServices []*api.Service
		fFQN := "." + f.GetPackage()

		// Messages
		for _, m := range f.MessageType {
			mFQN := fFQN + "." + m.GetName()
			if msg, ok := state.MessageByID[mFQN]; ok {
				result.Messages = append(result.Messages, msg)
			} else {
				slog.Warn("missing message in symbol table", "message", mFQN)
			}
		}

		// Enums
		for _, e := range f.EnumType {
			eFQN := fFQN + "." + e.GetName()
			if e, ok := state.EnumByID[eFQN]; ok {
				result.Enums = append(result.Enums, e)
			} else {
				slog.Warn("missing enum in symbol table", "message", eFQN)
			}
		}

		// Services
		for _, s := range f.Service {
			sFQN := fFQN + "." + s.GetName()
			service := processService(state, s, sFQN, f.GetPackage())
			for _, m := range s.Method {
				mFQN := sFQN + "." + m.GetName()
				if method := processMethod(state, m, mFQN, f.GetPackage()); method != nil {
					service.Methods = append(service.Methods, method)
				}
			}
			fileServices = append(fileServices, service)
		}

		// Add docs
		for _, loc := range f.GetSourceCodeInfo().GetLocation() {
			p := loc.GetPath()
			if loc.GetLeadingComments() == "" || len(p) == 0 {
				continue
			}

			switch p[0] {
			case fileDescriptorMessageType:
				// Because of message nesting we need to call recursively and
				// strip out parts of the path.
				m := f.MessageType[p[1]]
				addMessageDocumentation(state, m, p[2:], loc.GetLeadingComments(), fFQN+"."+m.GetName())
			case fileDescriptorEnumType:
				e := f.EnumType[p[1]]
				addEnumDocumentation(state, p[2:], loc.GetLeadingComments(), fFQN+"."+e.GetName())
			case fileDescriptorService:
				sFQN := fFQN + "." + f.GetService()[p[1]].GetName()
				addServiceDocumentation(state, p[2:], loc.GetLeadingComments(), sFQN)
			case fileDescriptorName, fileDescriptorPackage, fileDescriptorDependency,
				fileDescriptorExtension, fileDescriptorOptions, fileDescriptorSourceCodeInfo,
				fileDescriptorPublicDependency, fileDescriptorWeakDependency,
				fileDescriptorSyntax, fileDescriptorEdition:
				// We ignore this type of documentation because it produces no
				// output in the generated code.
			default:
				slog.Warn("dropped unknown documentation type", "loc", p, "docs", loc)
			}
		}
		result.Services = append(result.Services, fileServices...)
	}

	// Add the mixin methods to the existing services.
	for _, service := range result.Services {
		for _, f := range mixinFileDesc {
			fFQN := "." + f.GetPackage()
			for _, mixinProto := range f.Service {
				sFQN := fFQN + "." + mixinProto.GetName()
				mixin := processService(state, mixinProto, sFQN, f.GetPackage())
				for _, m := range mixinProto.Method {
					// We want to include the method in the existing service,
					// and not on the mixin.
					mFQN := service.ID + "." + m.GetName()
					originalFQN := sFQN + "." + m.GetName()
					if !enabledMixinMethods[originalFQN] {
						continue
					}
					if _, ok := state.MethodByID[mFQN]; ok {
						// The method already exists. This happens when services
						// require a mixin in the service config yaml *and* also
						// define the mixin method in the code.
						continue
					}
					if method := processMethod(state, m, mFQN, service.Package); method != nil {
						applyServiceConfigMethodOverrides(method, originalFQN, serviceConfig, result, mixin)
						service.Methods = append(service.Methods, method)
					}
				}
			}
		}
	}

	if result.Name == "" && serviceConfig != nil {
		result.Name = strings.TrimSuffix(serviceConfig.Name, ".googleapis.com")
	}
	updatePackageName(result)
	updateMethodPagination(result)
	updateAutoPopulatedFields(serviceConfig, result)
	return result
}

// requiresLongrunningMixin finds out if any method returns a LRO. This is used
// to forcibly load the longrunning mixin. It needs to happen before the proto
// descriptors are converted to the `api.*`, as that conversion requires the
// mixin.
func requiresLongrunningMixin(req *pluginpb.CodeGeneratorRequest) bool {
	for _, f := range req.GetSourceFileDescriptors() {
		for _, s := range f.Service {
			for _, m := range s.Method {
				info := parseOperationInfo(f.GetPackage(), m)
				if info != nil && m.GetOutputType() == ".google.longrunning.Operation" {
					return true
				}
			}
		}
	}
	return false
}

var descriptorpbToTypez = map[descriptorpb.FieldDescriptorProto_Type]api.Typez{
	descriptorpb.FieldDescriptorProto_TYPE_DOUBLE:   api.DOUBLE_TYPE,
	descriptorpb.FieldDescriptorProto_TYPE_FLOAT:    api.FLOAT_TYPE,
	descriptorpb.FieldDescriptorProto_TYPE_INT64:    api.INT64_TYPE,
	descriptorpb.FieldDescriptorProto_TYPE_UINT64:   api.UINT64_TYPE,
	descriptorpb.FieldDescriptorProto_TYPE_INT32:    api.INT32_TYPE,
	descriptorpb.FieldDescriptorProto_TYPE_FIXED64:  api.FIXED64_TYPE,
	descriptorpb.FieldDescriptorProto_TYPE_FIXED32:  api.FIXED32_TYPE,
	descriptorpb.FieldDescriptorProto_TYPE_BOOL:     api.BOOL_TYPE,
	descriptorpb.FieldDescriptorProto_TYPE_STRING:   api.STRING_TYPE,
	descriptorpb.FieldDescriptorProto_TYPE_BYTES:    api.BYTES_TYPE,
	descriptorpb.FieldDescriptorProto_TYPE_UINT32:   api.UINT32_TYPE,
	descriptorpb.FieldDescriptorProto_TYPE_SFIXED32: api.SFIXED32_TYPE,
	descriptorpb.FieldDescriptorProto_TYPE_SFIXED64: api.SFIXED64_TYPE,
	descriptorpb.FieldDescriptorProto_TYPE_SINT32:   api.SINT32_TYPE,
	descriptorpb.FieldDescriptorProto_TYPE_SINT64:   api.SINT64_TYPE,
	descriptorpb.FieldDescriptorProto_TYPE_GROUP:    api.GROUP_TYPE,
	descriptorpb.FieldDescriptorProto_TYPE_MESSAGE:  api.MESSAGE_TYPE,
	descriptorpb.FieldDescriptorProto_TYPE_ENUM:     api.ENUM_TYPE,
}

func normalizeTypes(state *api.APIState, in *descriptorpb.FieldDescriptorProto, field *api.Field) {
	typez := in.GetType()
	field.Typez = api.UNDEFINED_TYPE
	if tz, ok := descriptorpbToTypez[typez]; ok {
		field.Typez = tz
	}
	field.Repeated = in.Label != nil && *in.Label == descriptorpb.FieldDescriptorProto_LABEL_REPEATED

	switch typez {
	case descriptorpb.FieldDescriptorProto_TYPE_GROUP:
		field.TypezID = in.GetTypeName()
	case descriptorpb.FieldDescriptorProto_TYPE_MESSAGE:
		field.TypezID = in.GetTypeName()
		// Repeated fields are not optional, they can be empty, but always have
		// presence.
		field.Optional = !field.Repeated
		if message, ok := state.MessageByID[field.TypezID]; ok && message.IsMap {
			// Map fields appear as repeated in Protobuf. This is confusing,
			// as they typically are represented by a single `map<k, v>`-like
			// datatype. Protobuf leaks the wire-representation of maps, i.e.,
			// repeated pairs.
			field.Map = true
			field.Repeated = false
			field.Optional = false
		}
	case descriptorpb.FieldDescriptorProto_TYPE_ENUM:
		field.TypezID = in.GetTypeName()

	case
		descriptorpb.FieldDescriptorProto_TYPE_DOUBLE,
		descriptorpb.FieldDescriptorProto_TYPE_FLOAT,
		descriptorpb.FieldDescriptorProto_TYPE_INT64,
		descriptorpb.FieldDescriptorProto_TYPE_UINT64,
		descriptorpb.FieldDescriptorProto_TYPE_INT32,
		descriptorpb.FieldDescriptorProto_TYPE_FIXED64,
		descriptorpb.FieldDescriptorProto_TYPE_FIXED32,
		descriptorpb.FieldDescriptorProto_TYPE_BOOL,
		descriptorpb.FieldDescriptorProto_TYPE_STRING,
		descriptorpb.FieldDescriptorProto_TYPE_BYTES,
		descriptorpb.FieldDescriptorProto_TYPE_UINT32,
		descriptorpb.FieldDescriptorProto_TYPE_SFIXED32,
		descriptorpb.FieldDescriptorProto_TYPE_SFIXED64,
		descriptorpb.FieldDescriptorProto_TYPE_SINT32,
		descriptorpb.FieldDescriptorProto_TYPE_SINT64:
		// These do not need normalization
		return

	default:
		slog.Warn("found undefined field", "field", in.GetName())
	}
}

func processService(state *api.APIState, s *descriptorpb.ServiceDescriptorProto, sFQN, packagez string) *api.Service {
	service := &api.Service{
		Name:        s.GetName(),
		ID:          sFQN,
		Package:     packagez,
		DefaultHost: parseDefaultHost(s.GetOptions()),
		Deprecated:  s.GetOptions().GetDeprecated(),
	}
	state.ServiceByID[service.ID] = service
	return service
}

func processMethod(state *api.APIState, m *descriptorpb.MethodDescriptorProto, mFQN, packagez string) *api.Method {
	pathInfo, err := parsePathInfo(m, state)
	if err != nil {
		slog.Error("unsupported http method", "method", m, "error", err)
		return nil
	}
	routing, err := parseRoutingAnnotations(mFQN, m)
	if err != nil {
		slog.Error("cannot parse routing annotations", "method", m, "err", err)
		return nil
	}
	outputTypeID := m.GetOutputType()
	method := &api.Method{
		ID:                  mFQN,
		PathInfo:            pathInfo,
		Name:                m.GetName(),
		Deprecated:          m.GetOptions().GetDeprecated(),
		InputTypeID:         m.GetInputType(),
		OutputTypeID:        outputTypeID,
		ClientSideStreaming: m.GetClientStreaming(),
		ServerSideStreaming: m.GetServerStreaming(),
		OperationInfo:       parseOperationInfo(packagez, m),
		Routing:             routing,
		ReturnsEmpty:        outputTypeID == ".google.protobuf.Empty",
	}
	state.MethodByID[mFQN] = method
	return method
}

func processMessage(state *api.APIState, m *descriptorpb.DescriptorProto, mFQN, packagez string, parent *api.Message) *api.Message {
	message := &api.Message{
		Name:       m.GetName(),
		ID:         mFQN,
		Parent:     parent,
		Package:    packagez,
		Deprecated: m.GetOptions().GetDeprecated(),
	}
	state.MessageByID[mFQN] = message
	if opts := m.GetOptions(); opts != nil && opts.GetMapEntry() {
		message.IsMap = true
	}
	if len(m.GetNestedType()) > 0 {
		for _, nm := range m.GetNestedType() {
			nmFQN := mFQN + "." + nm.GetName()
			nmsg := processMessage(state, nm, nmFQN, packagez, message)
			message.Messages = append(message.Messages, nmsg)
		}
	}
	for _, e := range m.GetEnumType() {
		eFQN := mFQN + "." + e.GetName()
		e := processEnum(state, e, eFQN, packagez, message)
		message.Enums = append(message.Enums, e)
	}
	for _, oneof := range m.OneofDecl {
		oneOfs := &api.OneOf{
			Name: oneof.GetName(),
			ID:   mFQN + "." + oneof.GetName(),
		}
		message.OneOfs = append(message.OneOfs, oneOfs)
	}
	for _, mf := range m.Field {
		isProtoOptional := mf.Proto3Optional != nil && *mf.Proto3Optional
		field := &api.Field{
			Name:          mf.GetName(),
			ID:            mFQN + "." + mf.GetName(),
			JSONName:      mf.GetJsonName(),
			Deprecated:    mf.GetOptions().GetDeprecated(),
			Optional:      isProtoOptional,
			IsOneOf:       mf.OneofIndex != nil && !isProtoOptional,
			AutoPopulated: protobufIsAutoPopulated(mf),
			Behavior:      protobufFieldBehavior(mf),
		}
		normalizeTypes(state, mf, field)
		message.Fields = append(message.Fields, field)
		if field.IsOneOf {
			message.OneOfs[*mf.OneofIndex].Fields = append(message.OneOfs[*mf.OneofIndex].Fields, field)
		}
	}

	// Remove proto3 optionals from one-of
	var oneOfIdx int
	for _, oneof := range message.OneOfs {
		if len(oneof.Fields) > 0 {
			message.OneOfs[oneOfIdx] = oneof
			oneOfIdx++
		}
	}
	if oneOfIdx == 0 {
		message.OneOfs = nil
	} else {
		message.OneOfs = message.OneOfs[:oneOfIdx]
	}

	return message
}

func processEnum(state *api.APIState, e *descriptorpb.EnumDescriptorProto, eFQN, packagez string, parent *api.Message) *api.Enum {
	enum := &api.Enum{
		Name:       e.GetName(),
		ID:         eFQN,
		Parent:     parent,
		Package:    packagez,
		Deprecated: e.GetOptions().GetDeprecated(),
	}
	state.EnumByID[eFQN] = enum
	for _, ev := range e.Value {
		enumValue := &api.EnumValue{
			Name:       ev.GetName(),
			Number:     ev.GetNumber(),
			Parent:     enum,
			Deprecated: ev.GetOptions().GetDeprecated(),
		}
		enum.Values = append(enum.Values, enumValue)
	}
	numbers := map[int32]*api.EnumValue{}
	for _, v := range enum.Values {
		matchesStyle := func(v *api.EnumValue) bool {
			return strings.ToUpper(v.Name) == v.Name
		}
		if ev, ok := numbers[v.Number]; ok {
			if len(ev.Name) > len(v.Name) || (matchesStyle(v) && !matchesStyle(ev)) {
				numbers[v.Number] = v
			}
		} else {
			numbers[v.Number] = v
		}
	}
	unique := slices.Collect(maps.Values(numbers))
	slices.SortFunc(unique, func(a, b *api.EnumValue) int { return int(a.Number - b.Number) })
	enum.UniqueNumberValues = unique
	return enum
}

func addServiceDocumentation(state *api.APIState, p []int32, doc string, sFQN string) {
	switch {
	case len(p) == 0:
		// This is a comment for a service
		state.ServiceByID[sFQN].Documentation = trimLeadingSpacesInDocumentation(doc)
	case p[0] == serviceDescriptorProtoMethod && len(p) == 2:
		// This is a comment for a method
		state.ServiceByID[sFQN].Methods[p[1]].Documentation = trimLeadingSpacesInDocumentation(doc)
	case p[0] == serviceDescriptorProtoMethod:
		// A comment for something within a method (options, arguments, etc).
		// Ignored, as these comments do not refer to any artifact in the
		// generated code.
	case p[0] == serviceDescriptorProtoOption:
		// This is a comment for a service option. Ignored, as these comments do
		// not refer to any artifact in the generated code.
	default:
		slog.Warn("service dropped unknown documentation", "loc", p, "docs", doc)
	}
}

func addMessageDocumentation(state *api.APIState, m *descriptorpb.DescriptorProto, p []int32, doc string, mFQN string) {
	// Beware of refactoring the calls to `trimLeadingSpacesInDocumentation`.
	// We should modify `doc` only once, upon assignment to `.Documentation`
	switch {
	case len(p) == 0:
		// This is a comment for a top level message
		state.MessageByID[mFQN].Documentation = trimLeadingSpacesInDocumentation(doc)
	case p[0] == messageDescriptorNestedType:
		nmsg := m.GetNestedType()[p[1]]
		nmFQN := mFQN + "." + nmsg.GetName()
		addMessageDocumentation(state, nmsg, p[2:], doc, nmFQN)
	case p[0] == messageDescriptorField && len(p) == 2:
		state.MessageByID[mFQN].Fields[p[1]].Documentation = trimLeadingSpacesInDocumentation(doc)
	case p[0] == messageDescriptorEnum:
		eFQN := mFQN + "." + m.GetEnumType()[p[1]].GetName()
		addEnumDocumentation(state, p[2:], doc, eFQN)
	case p[0] == messageDescriptorOneOf && len(p) == 2:
		state.MessageByID[mFQN].OneOfs[p[1]].Documentation = trimLeadingSpacesInDocumentation(doc)
	case p[0] == messageDescriptorExtensionRange:
	case p[0] == messageDescriptorOptions:
	case p[0] == messageDescriptorExtension:
		// These comments are ignored, as they refer to Protobuf elements
		// without corresponding public APIs in the generated code.
	default:
		slog.Warn("message dropped documentation", "loc", p, "docs", doc)
	}
}

// addEnumDocumentation adds documentation to an enum.
func addEnumDocumentation(state *api.APIState, p []int32, doc string, eFQN string) {
	if len(p) == 0 {
		// This is a comment for an enum
		state.EnumByID[eFQN].Documentation = trimLeadingSpacesInDocumentation(doc)
	} else if len(p) == 2 && p[0] == enumDescriptorValue {
		state.EnumByID[eFQN].Values[p[1]].Documentation = trimLeadingSpacesInDocumentation(doc)
	} else {
		slog.Warn("enum dropped documentation", "loc", p, "docs", doc)
	}
}

// Protobuf removes the `//` leading characters, but leaves the leading
// whitespace. It is easier to reason about the comments in the rest of the
// generator if they are better normalized.
func trimLeadingSpacesInDocumentation(doc string) string {
	lines := strings.Split(doc, "\n")
	for i, line := range lines {
		lines[i] = strings.TrimPrefix(line, " ")
	}
	return strings.TrimSuffix(strings.Join(lines, "\n"), "\n")
}
