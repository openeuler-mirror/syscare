# SPDX-License-Identifier: GPL-2.0

if (DEFINED SYSCARE_VERSION)
    set(UPATCH_VERSION ${SYSCARE_VERSION})
    add_compile_options(-DUPATCH_VERSION="${UPATCH_VERSION}")
endif()

add_subdirectory(upatch-diff)
add_subdirectory(upatch-tool)
add_subdirectory(kmod)
