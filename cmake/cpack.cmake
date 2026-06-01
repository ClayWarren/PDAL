#------------------------------------------------------------------------------
# CPACK controls
#------------------------------------------------------------------------------

SET(CPACK_PACKAGE_VERSION_MAJOR ${PDAL_VERSION_MAJOR})
SET(CPACK_PACKAGE_VERSION_MINOR ${PDAL_VERSION_MINOR})
SET(CPACK_PACKAGE_VERSION_PATCH ${PDAL_VERSION_MINOR})
SET(CPACK_PACKAGE_NAME "PDAL")

SET(CPACK_SOURCE_GENERATOR "TBZ2;TGZ")
SET(CPACK_PACKAGE_VENDOR "PDAL Development Team")
SET(CPACK_RESOURCE_FILE_LICENSE    "${PROJECT_SOURCE_DIR}/LICENSE.txt")

set(CPACK_SOURCE_PACKAGE_FILE_NAME
    "${CMAKE_PROJECT_NAME}-${PDAL_VERSION}-src")

set(CPACK_SOURCE_IGNORE_FILES
    "/[.]gitattributes"
    "/[.]vagrant"
    "/[.]DS_Store"
    "/CVS/"
    "/[.]git/"
    "[.]swp$"
    "~$"
    "[.]#"
    "/#"
    "CMakeScripts/"
    "/[.]build/"
    "/[.]claude/"
    "CMakeCache.txt"
    "[.]xcodeproj"
    "build.make"
    "_CPack_Packages"
    "cmake_install.cmake"
    "Testing"
    "PDAL.build/"
    "/bin/"
    "/build[^/]*/"
    "/[.]mull-build/"
    "/[.]pixi/"
    "/rust/target/"
    "Makefile"
    "CMakeFiles"
    "CTestTestfile.cmake"
    "/test/data/local/"
    "/test/temp/"
    "/test/unit/TestConfig.hpp$"
    "/doc/doxygen/"
    "/doc/build/"
    "/doc/presentations/"
    "/doc/_static/logo/dongle/"
    "/cmake/examples/"
    "pdal_features.hpp$"
    "package.sh"
    "[.]gz2"
    "[.]bz2")

include(CPack)
