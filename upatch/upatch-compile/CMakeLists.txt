# SPDX-License-Identifier: GPL-2.0

add_subdirectory(ko)

add_executable(compiler-hijacker gnu-compiler-hijacker.c)
add_executable(as-hijacker gnu-as-hijacker.c)

# Copy compiler hijacker
add_custom_target(Copy_hijacker ALL
    COMMENT "Copying compiler hijacker..."
    COMMAND cp -a compiler-hijacker gcc-hijacker
    COMMAND cp -a compiler-hijacker g++-hijacker
    COMMAND cp -a compiler-hijacker cc-hijacker
    COMMAND cp -a compiler-hijacker c++-hijacker
    DEPENDS compiler-hijacker
    WORKING_DIRECTORY ${CMAKE_CURRENT_BINARY_DIR}
)

install(
    PROGRAMS
        ${CMAKE_CURRENT_BINARY_DIR}/gcc-hijacker
        ${CMAKE_CURRENT_BINARY_DIR}/g++-hijacker
        ${CMAKE_CURRENT_BINARY_DIR}/cc-hijacker
        ${CMAKE_CURRENT_BINARY_DIR}/c++-hijacker
    PERMISSIONS
        OWNER_EXECUTE OWNER_WRITE OWNER_READ
        GROUP_EXECUTE GROUP_READ
        WORLD_READ WORLD_EXECUTE
    DESTINATION
        ${SYSCARE_LIBEXEC_DIR}
)

install(
    TARGETS
        as-hijacker
    PERMISSIONS
        OWNER_EXECUTE OWNER_WRITE OWNER_READ
        GROUP_EXECUTE GROUP_READ
        WORLD_READ WORLD_EXECUTE
    DESTINATION
        ${SYSCARE_LIBEXEC_DIR}
)