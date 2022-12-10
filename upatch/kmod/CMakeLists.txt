# SPDX-License-Identifier: GPL-2.0

cmake_minimum_required(VERSION 3.14)
project(upatch-kmod)

set(UPATCH_KMOD "upatch.ko")
set(UPATCH_INSTALL_DIR /usr/libexec/syscare)

if (DEFINED KERNEL_VERSION)
    set(KERNEL_BUILD /lib/modules/${KERNEL_VERSION}/build)
    set(UPATCH_KMOD_CMD make UPATCH_VERSION=${UPATCH_VERSION} kernel=${KERNEL_BUILD})
else()
    set(UPATCH_KMOD_CMD make UPATCH_VERSION=${UPATCH_VERSION})
endif()

add_custom_target(upatch-kmod ALL
        COMMENT "Compiling upatch-mod..."
        BYPRODUCTS ${UPATCH_KMOD}
        COMMAND ${UPATCH_KMOD_CMD}
        WORKING_DIRECTORY ${CMAKE_CURRENT_SOURCE_DIR})

install(FILES ${UPATCH_KMOD} PERMISSIONS OWNER_WRITE OWNER_READ GROUP_READ WORLD_READ DESTINATION ${UPATCH_INSTALL_DIR})
