# Build hijackers
add_executable(gnu-as-hijacker gnu-as-hijacker.c)
add_executable(gnu-compiler-hijacker gnu-compiler-hijacker.c)

# Generate hijackers
add_custom_target(generate-upatch-hijackers ALL
    COMMENT "Generating upatch hijackers..."
    COMMAND ln -f gnu-as-hijacker as-hijacker
    COMMAND ln -f gnu-compiler-hijacker gcc-hijacker
    COMMAND ln -f gnu-compiler-hijacker g++-hijacker
    COMMAND ln -f gnu-compiler-hijacker cc-hijacker
    COMMAND ln -f gnu-compiler-hijacker c++-hijacker
    DEPENDS
        gnu-as-hijacker
        gnu-compiler-hijacker
    WORKING_DIRECTORY ${CMAKE_CURRENT_BINARY_DIR}
)

# Install hijackers
install(
    PROGRAMS
        ${CMAKE_CURRENT_BINARY_DIR}/gnu-as-hijacker
        ${CMAKE_CURRENT_BINARY_DIR}/gnu-compiler-hijacker
        ${CMAKE_CURRENT_BINARY_DIR}/as-hijacker
        ${CMAKE_CURRENT_BINARY_DIR}/gcc-hijacker
        ${CMAKE_CURRENT_BINARY_DIR}/g++-hijacker
        ${CMAKE_CURRENT_BINARY_DIR}/cc-hijacker
        ${CMAKE_CURRENT_BINARY_DIR}/c++-hijacker
    PERMISSIONS
        OWNER_READ OWNER_WRITE OWNER_EXECUTE
        GROUP_READ GROUP_EXECUTE
        WORLD_READ WORLD_EXECUTE
    DESTINATION
        ${SYSCARE_INSTALL_LIBEXEC_DIR}
)
