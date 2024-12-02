# Interface Definitions

A source of truth location for interface definition files. Including gRPC .proto files, as well as others like GraphQL.

This repo is meant, first, as a reference and second, as a landing place to get started communicating with service across the control system.

## Testing

With the protoc utility from https://github.com/protocolbuffers/protobuf
```
protoc --cpp_out=. $(find . -name "*.proto" -not -path "./include/*")
```
this command can be run to validate all the proto files in this repo.

## TODOs

- We should consider automations like building the binaries for gRPC for select languages.