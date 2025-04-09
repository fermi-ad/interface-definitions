# **gRPC and Protobuf Integration with Maven**

## Description

This project demonstrates how to integrate gRPC with Protocol Buffers (protobuf) using Maven. 
It automatically generates Java classes from .proto files and bundles them into a shaded JAR file.

## Prerequisites

* **Java 21:** Ensure Java is installed.
* **Maven:** The project uses Maven as a build tool. [Download Maven here.](https://maven.apache.org/download.cgi)
* **Protoc:** The Protocol Buffers compiler is needed to generate Java files from .proto files.


### Install Protoc:

* **Windows**: Download from [Protobuf releases](https://github.com/protocolbuffers/protobuf/releases), extract and add the folder containing protoc.exe to your PATH.

* **macOS**: Use Homebrew: brew install protobuf


## Installation

1. **Clone the Repository:** 
   `git clone https://github.com/fermi-ad/interface-definitions.git
   cd grpc-protobuf-example`

2. **Install Maven Dependencies:**
   `mvn install`

3. **Generate gRPC and Protobuf Files:**
   `mvn clean compile`

4. **Build the JAR:**
   `mvn clean package`


## Folder Structure

grpc-protobuf-example/
│
├── pom.xml                   # Maven build configuration
├── src/
│   ├── main/
│   │   ├── java/             # Java source code
│   │   └── proto/            # .proto files
│   ├── target/               # Build output (including generated sources)
├── README.md                 # Project documentation (this file)


## Troubleshooting

If you encounter errors like `RuntimeVersion$RuntimeDomain`, ensure that all necessary dependencies (like protobuf and gRPC) are included in your build.

**Verify that protoc is installed by running:**
`protoc --version`



