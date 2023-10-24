# SPDX-License-Identifier: GPL-2.0

set(UPATCH_TOOL_KMOD "upatch_manager.ko")

if (DEFINED KERNEL_VERSION)
    set(KERNEL_BUILD /lib/modules/${KERNEL_VERSION}/build)
    set(UPATCH_TOOL_KMOD_CMD make UPATCH_VERSION=${SYSCARE_VERSION} kernel=${KERNEL_BUILD})
else()
    set(UPATCH_TOOL_KMOD_CMD make UPATCH_VERSION=${SYSCARE_VERSION})
endif()
set(UPATHCH_TOOL_KMOD_CLEAN_CMD make clean)
add_custom_target(upatch-tool-kmod ALL
    COMMENT           "Compiling upatch-tool-kmod..."
    BYPRODUCTS        ${UPATCH_TOOL_KMOD}
    COMMAND           ${UPATHCH_TOOL_KMOD_CLEAN_CMD}
    COMMAND           ${UPATCH_TOOL_KMOD_CMD}
    WORKING_DIRECTORY ${CMAKE_CURRENT_SOURCE_DIR}
)

install(
    FILES
        ${UPATCH_TOOL_KMOD}
    PERMISSIONS
        OWNER_WRITE OWNER_READ
        GROUP_READ
        WORLD_READ
    DESTINATION
        ${SYSCARE_LIBEXEC_DIR}
)
