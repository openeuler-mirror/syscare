# SPDX-License-Identifier: GPL-2.0

include_directories(
    ../upatch-manage/
    ./
    ./insn/
)

file(GLOB HOST_SRC_FILES
    insn/*.c
    *.c
)

add_executable(upatch-diff ${HOST_SRC_FILES})
target_link_libraries(upatch-diff elf)

install(
    TARGETS
        upatch-diff
    PERMISSIONS
        OWNER_READ OWNER_WRITE OWNER_EXECUTE
        GROUP_READ GROUP_EXECUTE
        WORLD_READ WORLD_EXECUTE
    DESTINATION
        ${SYSCARE_LIBEXEC_DIR}
)
