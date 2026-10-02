cmake_minimum_required(VERSION 3.25)
project(MSIMEAppleBridgeTests LANGUAGES CXX OBJCXX)
if(PROJECT_IS_TOP_LEVEL)
  include(CTest)
endif()

# The session lease is the only source here; dictionary installation and snapshots go through the host-api C ABI.
if(BUILD_TESTING)
  add_executable(apple-bridge-session-lease
    tests/dictionary_session_lease.mm DictionarySessionLease.mm)
  target_compile_features(apple-bridge-session-lease PRIVATE cxx_std_17)
  target_include_directories(apple-bridge-session-lease PRIVATE "${CMAKE_CURRENT_SOURCE_DIR}")
  target_compile_options(apple-bridge-session-lease
    PRIVATE -fobjc-arc -Wall -Wextra -Werror -UNDEBUG)
  target_link_libraries(apple-bridge-session-lease PRIVATE "-framework Foundation")
  add_test(NAME apple-bridge-session-lease COMMAND apple-bridge-session-lease)
  set_tests_properties(apple-bridge-session-lease PROPERTIES TIMEOUT 30)
endif()
