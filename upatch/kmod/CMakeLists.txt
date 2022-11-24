# SPDX-License-Identifier: GPL-2.0

cmake_minimum_required(VERSION 3.14)
project(upatch-kmod)

set(UPATCH_KMOD "upatch.ko")
set(UPATCH_INSTALL_DIR /usr/libexec/syscare)

set(UPATCH_KMOD_CMD make)

add_custom_target(upatch-kmod ALL
        COMMENT "Compiling upatch-mod..."
        BYPRODUCTS ${UPATCH_KMOD}
        COMMAND ${UPATCH_KMOD_CMD}
        WORKING_DIRECTORY ${CMAKE_CURRENT_SOURCE_DIR})

install(FILES ${UPATCH_KMOD} PERMISSIONS OWNER_EXECUTE OWNER_WRITE OWNER_READ GROUP_EXECUTE GROUP_READ WORLD_READ WORLD_EXECUTE DESTINATION ${UPATCH_INSTALL_DIR})
