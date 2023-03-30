# SPDX-License-Identifier: GPL-2.0

add_compile_options(-DUPATCH_VERSION="${SYSCARE_VERSION}")

add_subdirectory(upatch-diff)
add_subdirectory(upatch-tool)
add_subdirectory(kmod)
