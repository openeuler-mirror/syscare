# SPDX-License-Identifier: GPL-2.0

add_compile_options(-DUPATCH_VERSION="${SYSCARE_VERSION}")

add_subdirectory(ko)
add_subdirectory(ebpf)

add_executable(upatch-gnu-compiler-hijacker gnu-compiler-hijacker.c)
add_executable(upatch-gnu-as-hijacker gnu-as-hijacker.c)

install(
    TARGETS
        upatch-gnu-compiler-hijacker
        upatch-gnu-as-hijacker
    PERMISSIONS
        OWNER_EXECUTE OWNER_WRITE OWNER_READ
        GROUP_EXECUTE GROUP_READ
        WORLD_READ WORLD_EXECUTE
    DESTINATION
        ${SYSCARE_LIBEXEC_DIR}
)