# SPDX-License-Identifier: GPL-2.0

add_compile_options(-DUPATCH_VERSION="${SYSCARE_VERSION}")
add_compile_options(-g -Wall -O2)

# Build upatch
add_custom_target(upatch ALL
    COMMENT "Compiling upatch..."
    COMMAND ${CMAKE_COMMAND} -E env
        "SYSCARE_VERSION=${SYSCARE_VERSION}"
        "UPATCH_VERSION=${SYSCARE_VERSION}"
        "RUSTFLAGS=--cfg unsound_local_offset"
        cargo build --release --target-dir ${CMAKE_CURRENT_BINARY_DIR}
    WORKING_DIRECTORY ${CMAKE_CURRENT_SOURCE_DIR}
)

add_subdirectory(upatch-diff)
add_subdirectory(upatch-manage)
add_subdirectory(upatch-compile)

# Install binaries
install(
    PROGRAMS
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
        ${CMAKE_CURRENT_SOURCE_DIR}/misc/upatch.service
    PERMISSIONS
        OWNER_EXECUTE OWNER_WRITE OWNER_READ
        GROUP_EXECUTE GROUP_READ
        WORLD_READ WORLD_EXECUTE
    DESTINATION
        ${SYSCARE_SERVICE_DIR}
)
