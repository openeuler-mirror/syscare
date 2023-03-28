# SPDX-License-Identifier: MulanPSL-2.0

cmake_minimum_required(VERSION 3.14)

project(syscare)

# Variables
SET(SYSCARE_VERSION     ${BUILD_VERSION})
SET(SYSCARE_BIN_DIR     ${CMAKE_INSTALL_PREFIX}/bin)
SET(SYSCARE_LIBEXEC_DIR ${CMAKE_INSTALL_PREFIX}/libexec/syscare)
SET(SYSCARE_SERVICE_DIR ${CMAKE_INSTALL_PREFIX}/lib/systemd/system)

message("-- SYSCARE_VERSION:     ${SYSCARE_VERSION}")
message("-- SYSCARE_BIN_DIR:     ${SYSCARE_BIN_DIR}")
message("-- SYSCARE_LIBEXEC_DIR: ${SYSCARE_LIBEXEC_DIR}")
message("-- SYSCARE_SERVICE_DIR: ${SYSCARE_SERVICE_DIR}")

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
        ${SYSCARE_BIN_DIR}
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
        ${PROJECT_SOURCE_DIR}/misc/syscare-pre.service
        ${PROJECT_SOURCE_DIR}/misc/syscare.service
    PERMISSIONS
        OWNER_EXECUTE OWNER_WRITE OWNER_READ
        GROUP_EXECUTE GROUP_READ
        WORLD_READ WORLD_EXECUTE
    DESTINATION
        ${SYSCARE_SERVICE_DIR}
)
