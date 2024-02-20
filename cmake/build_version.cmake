find_package(Git QUIET)

function(fetch_build_version VERSION_FILE OUTPUT_VARIABLE)
    # Get project version
    execute_process(
        COMMAND sh -c "cat ${VERSION_FILE} | grep -F 'version' | head -n 1 | awk -F '\"' '{print $2}'"
        OUTPUT_VARIABLE _PROJECT_VERSION
        OUTPUT_STRIP_TRAILING_WHITESPACE
        ERROR_QUIET
        WORKING_DIRECTORY ${CMAKE_CURRENT_LIST_DIR}
    )

    # Get git version
    if(GIT_FOUND)
        execute_process(
            COMMAND ${GIT_EXECUTABLE} rev-parse --short HEAD
            OUTPUT_VARIABLE _GIT_VERSION
            OUTPUT_STRIP_TRAILING_WHITESPACE
            ERROR_QUIET
            WORKING_DIRECTORY ${CMAKE_CURRENT_LIST_DIR}
        )
    endif()

    # Set build version
    if (DEFINED _GIT_VERSION)
        set(_BUILD_VERSION "${_PROJECT_VERSION}-${_GIT_VERSION}")
    else()
        set(_BUILD_VERSION "${_PROJECT_VERSION}")
    endif()

    # Set variables & compile options
    set(${OUTPUT_VARIABLE} ${_BUILD_VERSION} PARENT_SCOPE)
endfunction(fetch_build_version)
