# Find source files
file(GLOB_RECURSE SRC_FILES *.c)

# Build upatch-diff
add_executable(upatch-diff ${SRC_FILES})

target_include_directories(upatch-diff
    PRIVATE
        insn
)

target_link_libraries(upatch-diff
    PRIVATE
        elf
)

# Install upatch-diff
install(
    TARGETS
        upatch-diff
    PERMISSIONS
        OWNER_READ OWNER_WRITE OWNER_EXECUTE
        GROUP_READ GROUP_EXECUTE
        WORLD_READ WORLD_EXECUTE
    DESTINATION
        ${SYSCARE_INSTALL_LIBEXEC_DIR}
)
