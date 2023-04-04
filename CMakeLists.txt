# SPDX-License-Identifier: MulanPSL-2.0

cmake_minimum_required(VERSION 3.14)

project(syscare)

# Variables
set(SYSCARE_VERSION     ${BUILD_VERSION})
set(SYSCARE_BINARY_DIR  ${CMAKE_INSTALL_PREFIX}/bin)
set(SYSCARE_LIBEXEC_DIR ${CMAKE_INSTALL_PREFIX}/libexec/syscare)
set(SYSCARE_SERVICE_DIR ${CMAKE_INSTALL_PREFIX}/lib/systemd/system)

message("-- Syscare verion: ${SYSCARE_VERSION}")
message("-- Syscare binary  directory: ${SYSCARE_BINARY_DIR}")
message("-- Syscare libexec directory: ${SYSCARE_LIBEXEC_DIR}")
message("-- Syscare service directory: ${SYSCARE_SERVICE_DIR}")

# Build upatch
add_subdirectory(upatch)

# Build syscare
add_custom_target(syscare ALL
    COMMENT "Compiling syscare..."
    COMMAND cargo update -p clap --precise 4.0.32
    COMMAND cargo update -p clap_lex --precise 0.3.0
    COMMAND UPATCH_VERSION=${SYSCARE_VERSION}; SYSCARE_VERSION=${SYSCARE_VERSION}; cargo build --release --target-dir ${CMAKE_CURRENT_BINARY_DIR}
    WORKING_DIRECTORY ${CMAKE_CURRENT_SOURCE_DIR}
)

# Install binaries
install(
    PROGRAMS
        ${CMAKE_CURRENT_BINARY_DIR}/release/syscare
    PERMISSIONS
        OWNER_EXECUTE OWNER_WRITE OWNER_READ
        GROUP_EXECUTE GROUP_READ
        WORLD_READ WORLD_EXECUTE
    DESTINATION
        ${SYSCARE_BINARY_DIR}
)

install(
    PROGRAMS
        ${CMAKE_CURRENT_BINARY_DIR}/release/upatch-build
        ${CMAKE_CURRENT_BINARY_DIR}/release/syscare-build
    PERMISSIONS
        OWNER_EXECUTE OWNER_WRITE OWNER_READ
        GROUP_EXECUTE GROUP_READ
        WORLD_READ WORLD_EXECUTE
    DESTINATION
        ${SYSCARE_LIBEXEC_DIR}
)

# Install service
install(
    FILES
        ${PROJECT_SOURCE_DIR}/misc/syscare.service
    PERMISSIONS
        OWNER_EXECUTE OWNER_WRITE OWNER_READ
        GROUP_EXECUTE GROUP_READ
        WORLD_READ WORLD_EXECUTE
    DESTINATION
        ${SYSCARE_SERVICE_DIR}
)
