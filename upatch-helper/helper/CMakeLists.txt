# Build helpers
add_executable(gnu-as-helper gnu-as-helper.c)
add_executable(gnu-compiler-helper gnu-compiler-helper.c)

# Generate helpers
add_custom_target(generate-upatch-helpers ALL
    COMMENT "Generating upatch helpers..."
    COMMAND ln -f gnu-as-helper as-helper
    COMMAND ln -f gnu-compiler-helper gcc-helper
    COMMAND ln -f gnu-compiler-helper g++-helper
    COMMAND ln -f gnu-compiler-helper cc-helper
    COMMAND ln -f gnu-compiler-helper c++-helper
    DEPENDS
        gnu-as-helper
        gnu-compiler-helper
    WORKING_DIRECTORY ${CMAKE_CURRENT_BINARY_DIR}
)

# Install helpers
install(
    PROGRAMS
        ${CMAKE_CURRENT_BINARY_DIR}/gnu-as-helper
        ${CMAKE_CURRENT_BINARY_DIR}/gnu-compiler-helper
        ${CMAKE_CURRENT_BINARY_DIR}/as-helper
        ${CMAKE_CURRENT_BINARY_DIR}/gcc-helper
        ${CMAKE_CURRENT_BINARY_DIR}/g++-helper
        ${CMAKE_CURRENT_BINARY_DIR}/cc-helper
        ${CMAKE_CURRENT_BINARY_DIR}/c++-helper
    PERMISSIONS
        OWNER_READ OWNER_WRITE OWNER_EXECUTE
        GROUP_READ GROUP_EXECUTE
        WORLD_READ WORLD_EXECUTE
    DESTINATION
        ${SYSCARE_LIBEXEC_DIR}
)
