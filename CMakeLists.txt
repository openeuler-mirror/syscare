# SPDX-License-Identifier: MulanPSL-2.0

cmake_minimum_required(VERSION 3.14)
find_package(Git QUIET)

project(syscare)

add_compile_options(-g -Wall -O2)

# Variables
if(NOT DEFINED BUILD_VERSION)
    set(BUILD_VERSION "dev")
endif()

if(GIT_FOUND)
    execute_process(
        COMMAND ${GIT_EXECUTABLE} rev-parse --short HEAD
        OUTPUT_VARIABLE GIT_VERSION
        OUTPUT_STRIP_TRAILING_WHITESPACE
        ERROR_QUIET
        WORKING_DIRECTORY ${CMAKE_CURRENT_LIST_DIR}
    )
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

# Build upatch
add_subdirectory(upatch)

# Build syscare
add_custom_target(syscare ALL
    COMMENT "Compiling syscare..."
    COMMAND UPATCH_VERSION=${SYSCARE_VERSION}; SYSCARE_VERSION=${SYSCARE_VERSION}; cargo build --release --target-dir ${CMAKE_CURRENT_BINARY_DIR}
    WORKING_DIRECTORY ${CMAKE_CURRENT_SOURCE_DIR}
)

# Install binaries
install(
    PROGRAMS
        ${CMAKE_CURRENT_BINARY_DIR}/release/syscare
        ${CMAKE_CURRENT_BINARY_DIR}/release/syscared
        ${CMAKE_CURRENT_BINARY_DIR}/release/upatchd
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
