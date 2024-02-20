cmake_minimum_required(VERSION 3.14)

project(syscare)

# Includes
include(cmake/build_option.cmake)
include(cmake/build_version.cmake)
include(cmake/install_directory.cmake)

# Build version
fetch_build_version("syscare/Cargo.toml" BUILD_VERSION)
add_compile_options(-DBUILD_VERSION="${BUILD_VERSION}")

# Build info
message("---------------------------------------------------------")
message("███████╗██╗   ██╗███████╗ ██████╗ █████╗ ██████╗ ███████╗")
message("██╔════╝╚██╗ ██╔╝██╔════╝██╔════╝██╔══██╗██╔══██╗██╔════╝")
message("███████╗ ╚████╔╝ ███████╗██║     ███████║██████╔╝█████╗  ")
message("╚════██║  ╚██╔╝  ╚════██║██║     ██╔══██║██╔══██╗██╔══╝  ")
message("███████║   ██║   ███████║╚██████╗██║  ██║██║  ██║███████╗")
message("╚══════╝   ╚═╝   ╚══════╝ ╚═════╝╚═╝  ╚═╝╚═╝  ╚═╝╚══════╝")
message("---------------------------------------------------------")
message("-- Verion: ${BUILD_VERSION}")
message("-- Binary  directory: ${SYSCARE_INSTALL_BINARY_DIR}")
message("-- Libexec directory: ${SYSCARE_INSTALL_LIBEXEC_DIR}")
message("-- Service directory: ${SYSCARE_INSTALL_SERVICE_DIR}")
message("---------------------------------------------------------")

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
        OWNER_READ OWNER_WRITE OWNER_EXECUTE
        GROUP_READ GROUP_EXECUTE
        WORLD_READ WORLD_EXECUTE
    DESTINATION
        ${SYSCARE_INSTALL_BINARY_DIR}
)

install(
    PROGRAMS
        ${CMAKE_CURRENT_BINARY_DIR}/release/upatch-build
        ${CMAKE_CURRENT_BINARY_DIR}/release/syscare-build
    PERMISSIONS
        OWNER_READ OWNER_WRITE OWNER_EXECUTE
        GROUP_READ GROUP_EXECUTE
        WORLD_READ WORLD_EXECUTE
    DESTINATION
        ${SYSCARE_INSTALL_LIBEXEC_DIR}
)

# Build other components
add_subdirectory(upatch-diff)
add_subdirectory(upatch-manage)
add_subdirectory(upatch-hijacker)
add_subdirectory(misc)
