(using)=

# CMake

This tutorial will explain how to use PDAL in your own projects using CMake. A
more complete, working example can be found {ref}`here <writing>`.

```{note}
We assume you have either {ref}`built or installed<building>` PDAL.
```

## Basic CMake configuration

Begin by creating a file named CMakeLists.txt that contains:

```cmake
cmake_minimum_required(VERSION 3.13)
project(MY_PDAL_PROJECT LANGUAGES CXX)
find_package(PDAL 2.0.0 REQUIRED CONFIG)
add_executable(tutorial tutorial.cpp)
target_link_libraries(tutorial PRIVATE PDAL::PDAL)
```

## CMakeLists explained

```cmake
cmake_minimum_required(VERSION 3.13)
```

The `cmake_minimum_required` command specifies the minimum required version of
CMake.

```cmake
project(MY_PDAL_PROJECT LANGUAGES CXX)
```

The CMake `project` command names your project and sets a number of useful
CMake variables.

```cmake
find_package(PDAL 2.0.0 REQUIRED CONFIG)
```

We next ask CMake to locate the PDAL package, requiring version 2.0.0 or higher.

```cmake
target_link_libraries(tutorial PRIVATE PDAL::PDAL)
```

If PDAL is found, it provides the imported `PDAL::PDAL` target. Link your
executable or library to that target so CMake receives the installed include
directories, compile features, and link dependencies from PDAL's package
configuration.

```cmake
add_executable(tutorial tutorial.cpp)
```

We use the `add_executable` command to tell CMake to create an executable named
`tutorial` from the source file `tutorial.cpp`.

```cmake
target_link_libraries(tutorial PRIVATE PDAL::PDAL)
```

We assume that the tutorial executable makes calls to PDAL functions. To make
the linker aware of the PDAL libraries, we use `target_link_libraries` to link
`tutorial` against the imported `PDAL::PDAL` target.

## Compiling the project

Make a `build` directory, where compilation will occur:

```bash
$ cd /PATH/TO/MY/PDAL/PROJECT
$ mkdir build
```

Run cmake from within the build directory:

```bash
$ cd build
$ cmake ..
```

Now, build the project:

```bash
$ make
```

The project is now built and ready to run:

```bash
$ ./tutorial
```
