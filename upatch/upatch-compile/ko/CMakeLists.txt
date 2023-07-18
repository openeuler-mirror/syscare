# SPDX-License-Identifier: GPL-2.0

set(UPATCH_HIJACKER_KMOD "upatch-hijacker.ko")

if (DEFINED KERNEL_VERSION)
    set(KERNEL_BUILD /lib/modules/${KERNEL_VERSION}/build)
    set(UPATCH_HIJACKER_KMOD_CMD make UPATCH_VERSION=${SYSCARE_VERSION} kernel=${KERNEL_BUILD})
else()
    set(UPATCH_HIJACKER_KMOD_CMD make UPATCH_VERSION=${SYSCARE_VERSION})
endif()

add_custom_target(upatch-kmod ALL
    COMMENT           "Compiling upatch-mod..."
    BYPRODUCTS        ${UPATCH_HIJACKER_KMOD}
    COMMAND           ${UPATCH_HIJACKER_KMOD_CMD}
    WORKING_DIRECTORY ${CMAKE_CURRENT_SOURCE_DIR}
)

install(
    FILES
        ${UPATCH_HIJACKER_KMOD}
    PERMISSIONS
        OWNER_WRITE OWNER_READ
        GROUP_READ
        WORLD_READ
    DESTINATION
        ${SYSCARE_LIBEXEC_DIR}
)