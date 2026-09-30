# Builds the golden recorder against an already-built reference engine (see README.md).
# Mirrors how the reference links its test executables (MSIME-Engine/CMakeLists.txt: target_link_libraries(<test> PRIVATE MetasequoiaIme::Engine)) and the exact flags of build-ref/CMakeFiles/eval_sentences.dir/{flags.make,link.txt}.
cmake_minimum_required(VERSION 3.25)
project(MsimeEngineGolden LANGUAGES CXX)

set(CMAKE_CXX_STANDARD 17)
set(CMAKE_CXX_STANDARD_REQUIRED ON)
set(CMAKE_CXX_EXTENSIONS OFF)
if(NOT CMAKE_BUILD_TYPE)
    set(CMAKE_BUILD_TYPE Release)
endif()

set(MSIME_ENGINE_REF_SRC "" CACHE PATH "Reference MSIME-Engine source tree (pinned archive + overlays)")
set(MSIME_ENGINE_REF_BUILD "" CACHE PATH "Configured and built reference tree containing libMetasequoiaImeEngine.a")
if(NOT EXISTS "${MSIME_ENGINE_REF_SRC}/include/metasequoia/session.h")
    message(FATAL_ERROR "MSIME_ENGINE_REF_SRC must point at the reference MSIME-Engine source tree")
endif()
if(NOT EXISTS "${MSIME_ENGINE_REF_BUILD}/libMetasequoiaImeEngine.a")
    message(FATAL_ERROR "MSIME_ENGINE_REF_BUILD must contain libMetasequoiaImeEngine.a")
endif()

# Same dependency set the engine target exports (Boost headers, header-only fmt, SQLite3, Threads).
find_package(Boost REQUIRED)
find_package(SQLite3 REQUIRED)
find_package(Threads REQUIRED)
find_package(nlohmann_json 3 CONFIG REQUIRED)

add_library(MetasequoiaImeEngineRef STATIC IMPORTED)
set_target_properties(MetasequoiaImeEngineRef PROPERTIES
    IMPORTED_LOCATION "${MSIME_ENGINE_REF_BUILD}/libMetasequoiaImeEngine.a"
    INTERFACE_INCLUDE_DIRECTORIES "${MSIME_ENGINE_REF_SRC};${MSIME_ENGINE_REF_SRC}/include"
    INTERFACE_COMPILE_DEFINITIONS "FMT_HEADER_ONLY=1")
if(TARGET SQLite3::SQLite3)
    set(GOLDEN_SQLITE SQLite3::SQLite3)
else()
    set(GOLDEN_SQLITE SQLite::SQLite3)
endif()
target_link_libraries(MetasequoiaImeEngineRef INTERFACE Boost::headers ${GOLDEN_SQLITE} Threads::Threads)

add_executable(engine_golden_recorder recorder.cpp)
target_link_libraries(engine_golden_recorder PRIVATE MetasequoiaImeEngineRef nlohmann_json::nlohmann_json)
