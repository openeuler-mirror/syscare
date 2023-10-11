# SPDX-License-Identifier: MulanPSL-2.0

cmake_minimum_required(VERSION 3.14)
find_package(Git QUIET)

project(syscare)

# Variables
if(NOT DEFINED BUILD_VERSION)
    set(BUILD_VERSION "dev")
endif()

if(NOT DEFINED _BUILD_UPATCH)
    set(_BUILD_UPATCH 1)
endif()

if(GIT_FOUND)
    execute_process(
        COMMAND ${GIT_EXECUTABLE} rev-parse --short HEAD
        OUTPUT_VARIABLE GIT_VERSION
        OUTPUT_STRIP_TRAILING_WHITESPACE
        ERROR_QUIET
        WORKING_DIRECTORY ${CMAKE_CURRENT_LIST_DIR}
    )
endif()

if(DEFINED GIT_VERSION)
    set(SYSCARE_VERSION "${BUILD_VERSION}-${GIT_VERSION}")
else()
    set(SYSCARE_VERSION "${BUILD_VERSION}")
endif()

set(SYSCARE_BINARY_DIR  "${CMAKE_INSTALL_PREFIX}/bin")
set(SYSCARE_LIBEXEC_DIR "${CMAKE_INSTALL_PREFIX}/libexec/syscare")
set(SYSCARE_SERVICE_DIR "${CMAKE_INSTALL_PREFIX}/lib/systemd/system")

message("-- Syscare verion: ${SYSCARE_VERSION}")
message("-- Syscare binary  directory: ${SYSCARE_BINARY_DIR}")
message("-- Syscare libexec directory: ${SYSCARE_LIBEXEC_DIR}")
message("-- Syscare service directory: ${SYSCARE_SERVICE_DIR}")

# Build syscare
add_custom_target(syscare ALL
    COMMENT "Compiling syscare..."
    COMMAND ${CMAKE_COMMAND} -E env
        "SYSCARE_VERSION=${SYSCARE_VERSION}"
        "UPATCH_VERSION=${SYSCARE_VERSION}"
        "RUSTFLAGS=--cfg unsound_local_offset"
        cargo build --release --target-dir ${CMAKE_CURRENT_BINARY_DIR}
    WORKING_DIRECTORY ${CMAKE_CURRENT_SOURCE_DIR}
)

# Build upatch
if(${_BUILD_UPATCH})
    add_subdirectory(upatch)
endif()

# Install binaries
install(
    PROGRAMS
        ${CMAKE_CURRENT_BINARY_DIR}/release/syscare
        ${CMAKE_CURRENT_BINARY_DIR}/release/syscared
    PERMISSIONS
        OWNER_EXECUTE OWNER_WRITE OWNER_READ
        GROUP_EXECUTE GROUP_READ
        WORLD_READ WORLD_EXECUTE
    DESTINATION
        ${SYSCARE_BINARY_DIR}
)

install(
    PROGRAMS
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
        ${CMAKE_CURRENT_LIST_DIR}/misc/syscare.service
    PERMISSIONS
        OWNER_EXECUTE OWNER_WRITE OWNER_READ
        GROUP_EXECUTE GROUP_READ
        WORLD_READ WORLD_EXECUTE
    DESTINATION
        ${SYSCARE_SERVICE_DIR}
)
