# SPDX-License-Identifier: GPL-2.0

include_directories(
    ../kmod
    ./
)

file(GLOB TOOL_SRC_FILES
    *.c
)

add_executable(upatch-tool ${TOOL_SRC_FILES})

install(
    TARGETS
        upatch-tool
    PERMISSIONS
        OWNER_EXECUTE OWNER_WRITE OWNER_READ
        GROUP_EXECUTE GROUP_READ
        WORLD_READ WORLD_EXECUTE
    DESTINATION
        ${SYSCARE_LIBEXEC_DIR}
)
