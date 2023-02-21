# SPDX-License-Identifier: MulanPSL2

cmake_minimum_required(VERSION 3.14)
project(syscare-cli)

set(SYSCARE_INSTALL_DIR /usr/bin)

set(CARGO_CMD SYSCARE_BUILD_VERSION=${SYSCARE_BUILD_VERSION} cargo build --release)
set(TARGET_DIR "release")

set(SYSCARE_CLI_BIN "syscare")

add_custom_target(syscare-cli ALL
        COMMENT "Compiling syscare-cli..."
        COMMAND cargo update -p clap --precise 4.0.32
        COMMAND cargo update -p clap_lex --precise 0.3.0
        COMMAND CARGO_TARGET_DIR=${CMAKE_CURRENT_BINARY_DIR} ${CARGO_CMD}
        COMMAND cp ${CMAKE_CURRENT_BINARY_DIR}/${TARGET_DIR}/${SYSCARE_CLI_BIN} ${CMAKE_CURRENT_BINARY_DIR}
        WORKING_DIRECTORY ${CMAKE_CURRENT_SOURCE_DIR})

install(FILES ${CMAKE_CURRENT_BINARY_DIR}/${SYSCARE_CLI_BIN} PERMISSIONS OWNER_EXECUTE OWNER_WRITE OWNER_READ GROUP_EXECUTE GROUP_READ WORLD_READ WORLD_EXECUTE  DESTINATION ${SYSCARE_INSTALL_DIR})
