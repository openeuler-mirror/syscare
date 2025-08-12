# SPDX-License-Identifier: GPL-2.0

set(UPATCH_MANAGE_KMOD "upatch_manage.ko")

if (DEFINED KERNEL_SRC)
    set(KMOD_KERNEL "KERNEL_SRC=${KERNEL_SRC}")
endif()

if (DEFINED BUILD_VERSION)
    set(KMOD_VERSION "MODULE_VERSION=${BUILD_VERSION}")
endif()

if (DEFINED ENABLE_GCOV)
    set(KMOD_GCOV "GCOV=1")
endif()

add_custom_target(upatch-manage ALL
    COMMAND make -C ${CMAKE_CURRENT_SOURCE_DIR} ${KMOD_KERNEL} ${KMOD_VERSION} ${KMOD_GCOV}
)

install(
    FILES
        ${UPATCH_MANAGE_KMOD}
    PERMISSIONS
        OWNER_WRITE OWNER_READ
        GROUP_READ
        WORLD_READ
    DESTINATION
        ${SYSCARE_LIBEXEC_DIR}
)
