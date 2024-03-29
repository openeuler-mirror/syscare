# SPDX-License-Identifier: MulanPSL-2.0

cmake_minimum_required(VERSION 3.14)

project(syscare)

# Includes
include(GNUInstallDirs)
find_package(Git QUIET)

# Version
if(NOT DEFINED BUILD_VERSION)
    execute_process(
        COMMAND sh -c "cat syscare/Cargo.toml | grep -F 'version' | head -n 1 | awk -F '\"' '{print $2}'"
        OUTPUT_VARIABLE BUILD_VERSION
        OUTPUT_STRIP_TRAILING_WHITESPACE
        ERROR_QUIET
        WORKING_DIRECTORY ${CMAKE_CURRENT_LIST_DIR}
    )
endif()

if(GIT_FOUND)
    execute_process(
        COMMAND ${GIT_EXECUTABLE} rev-parse --short HEAD
        OUTPUT_VARIABLE GIT_VERSION
        OUTPUT_STRIP_TRAILING_WHITESPACE
        ERROR_QUIET
        WORKING_DIRECTORY ${CMAKE_CURRENT_LIST_DIR}
    )
    set(BUILD_VERSION "${BUILD_VERSION}-g${GIT_VERSION}")
else()
    set(BUILD_VERSION "${BUILD_VERSION}")
endif()

# Set install directories
set(SYSCARE_BINARY_DIR  "${CMAKE_INSTALL_FULL_BINDIR}")
set(SYSCARE_LIBEXEC_DIR "${CMAKE_INSTALL_FULL_LIBEXECDIR}/syscare")
set(SYSCARE_SERVICE_DIR "${CMAKE_INSTALL_PREFIX}/lib/systemd/system")

# Print build info
message("---------------------------------------------------------")
message("███████╗██╗   ██╗███████╗ ██████╗ █████╗ ██████╗ ███████╗")
message("██╔════╝╚██╗ ██╔╝██╔════╝██╔════╝██╔══██╗██╔══██╗██╔════╝")
message("███████╗ ╚████╔╝ ███████╗██║     ███████║██████╔╝█████╗  ")
message("╚════██║  ╚██╔╝  ╚════██║██║     ██╔══██║██╔══██╗██╔══╝  ")
message("███████║   ██║   ███████║╚██████╗██║  ██║██║  ██║███████╗")
message("╚══════╝   ╚═╝   ╚══════╝ ╚═════╝╚═╝  ╚═╝╚═╝  ╚═╝╚══════╝")
message("---------------------------------------------------------")
message("-- Verion: ${BUILD_VERSION}")
message("-- Binary  directory: ${SYSCARE_BINARY_DIR}")
message("-- Libexec directory: ${SYSCARE_LIBEXEC_DIR}")
message("-- Service directory: ${SYSCARE_SERVICE_DIR}")
message("---------------------------------------------------------")

# Compile options
add_compile_options(-DBUILD_VERSION="${BUILD_VERSION}")
add_compile_options(-g -Wall -O2 -fPIE)

# Subdirectories
add_subdirectory(upatch-diff)
add_subdirectory(upatch-manage)
add_subdirectory(upatch-hijacker)
add_subdirectory(misc)

# Build rust executables
add_custom_target(rust-executables ALL
    COMMENT "Building rust executables..."
    COMMAND ${CMAKE_COMMAND} -E env
        "BUILD_VERSION=${BUILD_VERSION}"
        "RUSTFLAGS=--cfg unsound_local_offset"
        cargo build --release --target-dir ${CMAKE_CURRENT_BINARY_DIR}
    WORKING_DIRECTORY ${CMAKE_CURRENT_SOURCE_DIR}
)

# Install rust binaries
install(
    PROGRAMS
        ${CMAKE_CURRENT_BINARY_DIR}/release/upatchd
        ${CMAKE_CURRENT_BINARY_DIR}/release/syscared
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
