/******************************************************************************
 * Copyright (c) 2021, Hobu Inc. (info@hobu.co)
 *
 * All rights reserved.
 *
 * Redistribution and use in source and binary forms, with or without
 * modification, are permitted provided that the following
 * conditions are met:
 *
 *     * Redistributions of source code must retain the above copyright
 *       notice, this list of conditions and the following disclaimer.
 *     * Redistributions in binary form must reproduce the above copyright
 *       notice, this list of conditions and the following disclaimer in
 *       the documentation and/or other materials provided
 *       with the distribution.
 *     * Neither the name of Hobu, Inc. or Flaxen Geo Consulting nor the
 *       names of its contributors may be used to endorse or promote
 *       products derived from this software without specific prior
 *       written permission.
 *
 * THIS SOFTWARE IS PROVIDED BY THE COPYRIGHT HOLDERS AND CONTRIBUTORS
 * "AS IS" AND ANY EXPRESS OR IMPLIED WARRANTIES, INCLUDING, BUT NOT
 * LIMITED TO, THE IMPLIED WARRANTIES OF MERCHANTABILITY AND FITNESS
 * FOR A PARTICULAR PURPOSE ARE DISCLAIMED. IN NO EVENT SHALL THE
 * COPYRIGHT OWNER OR CONTRIBUTORS BE LIABLE FOR ANY DIRECT, INDIRECT,
 * INCIDENTAL, SPECIAL, EXEMPLARY, OR CONSEQUENTIAL DAMAGES (INCLUDING,
 * BUT NOT LIMITED TO, PROCUREMENT OF SUBSTITUTE GOODS OR SERVICES; LOSS
 * OF USE, DATA, OR PROFITS; OR BUSINESS INTERRUPTION) HOWEVER CAUSED
 * AND ON ANY THEORY OF LIABILITY, WHETHER IN CONTRACT, STRICT LIABILITY,
 * OR TORT (INCLUDING NEGLIGENCE OR OTHERWISE) ARISING IN ANY WAY OUT
 * OF THE USE OF THIS SOFTWARE, EVEN IF ADVISED OF THE POSSIBILITY
 * OF SUCH DAMAGE.
 ****************************************************************************/

#pragma once

#include <set>
#include <stdint.h>
#include <string>
#include <vector>

#include <pdal/SpatialReference.hpp>

#include <io/HeaderVal.hpp>

#include <io/private/las/Utils.hpp>
#include <io/private/las/Vlr.hpp>

namespace pdal
{

namespace copcwriter
{

// The COPC writer stage (io/CopcWriter.cpp) parses its options into these
// structs, then delegates the octree build and file assembly to the Rust
// writers.copc through the C ABI. Nothing here performs the actual write.

struct Options
{
    StringList forwardSpec;
    StringList extraDimSpec;
    NumHeaderVal<uint16_t, 0, 65535> filesourceId;
    NumHeaderVal<uint16_t, 0, 31> globalEncoding;
    UuidHeaderVal projectId;
    StringHeaderVal<32> systemId;
    StringHeaderVal<32> softwareId;
    NumHeaderVal<uint16_t, 0, 366> creationDoy;
    NumHeaderVal<uint16_t, 0, 65535> creationYear;
    StringHeaderVal<0> scaleX;
    StringHeaderVal<0> scaleY;
    StringHeaderVal<0> scaleZ;
    StringHeaderVal<0> offsetX;
    StringHeaderVal<0> offsetY;
    StringHeaderVal<0> offsetZ;
    std::vector<las::Evlr> userVlrs;
    bool emitPipeline;
    bool emitMetadata;
    bool fixedSeed;
    pdal::SpatialReference aSrs;
    int threadCount = 10;
    bool enhancedSrsVlrs = false;
};

struct BaseInfo
{
    Options opts;
    SpatialReference srs;
    int pointFormatId;
    las::ExtraDims extraDims;
    int numExtraBytes;
    std::set<std::string> forwards;
    bool forwardVlrs = false;
    int viewCount = 0;
    std::vector<las::Evlr> vlrs;
};

} // namespace copcwriter
} // namespace pdal
