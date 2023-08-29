# SPDX-License-Identifier: GPL-2.0

add_compile_options(-DUPATCH_VERSION="${SYSCARE_VERSION}")

add_subdirectory(upatch-diff)
add_subdirectory(upatch-tool)
add_subdirectory(upatch-manage)
add_subdirectory(upatch-compile)

# Install service
install(
    FILES
        ${PROJECT_SOURCE_DIR}/misc/syscare-upatch.service
    PERMISSIONS
        OWNER_EXECUTE OWNER_WRITE OWNER_READ
        GROUP_EXECUTE GROUP_READ
        WORLD_READ WORLD_EXECUTE
    DESTINATION
        ${SYSCARE_SERVICE_DIR}
)
