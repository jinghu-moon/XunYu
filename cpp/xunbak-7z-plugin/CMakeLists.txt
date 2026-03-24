cmake_minimum_required(VERSION 3.25)

project(xunbak_7z_plugin LANGUAGES CXX)

set(CMAKE_CXX_STANDARD 20)
set(CMAKE_CXX_STANDARD_REQUIRED ON)
set(CMAKE_CXX_EXTENSIONS OFF)
set(CMAKE_MSVC_RUNTIME_LIBRARY "MultiThreadedDLL")

set(XUNBAK_PLUGIN_HEADERS_DIR "${CMAKE_CURRENT_SOURCE_DIR}/../../refer/7z-assembly-master/src/plugin")
set(XUNBAK_7Z_CORE_LIB
    "${CMAKE_CURRENT_SOURCE_DIR}/../../target/debug/xunbak_7z_core.lib"
    CACHE FILEPATH
    "Path to the xunbak_7z_core static library")

if (NOT EXISTS "${XUNBAK_PLUGIN_HEADERS_DIR}/Export.h")
    message(FATAL_ERROR "7-Zip plugin headers not found: ${XUNBAK_PLUGIN_HEADERS_DIR}")
endif ()

add_library(xunbak SHARED
    xunbak_exports.cpp
    xunbak_handler.cpp
    xunbak_utils.cpp
)

target_include_directories(xunbak PRIVATE
    "${CMAKE_CURRENT_SOURCE_DIR}"
    "${XUNBAK_PLUGIN_HEADERS_DIR}"
)

target_compile_definitions(xunbak PRIVATE UNICODE _UNICODE NOMINMAX)
target_link_libraries(xunbak PRIVATE "${XUNBAK_7Z_CORE_LIB}" OleAut32)
target_link_libraries(xunbak PRIVATE
    Ws2_32
    Userenv
    Iphlpapi
    Propsys
    RuntimeObject
    Ntdll
)

set_target_properties(xunbak PROPERTIES
    OUTPUT_NAME "xunbak"
)
