/******************************************************************************
 * Copyright (c) 2014, Howard Butler, hobu.inc@gmail.com
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

#include <stdint.h>
#include <string>
#include <vector>

#include <pdal/Dimension.hpp>
#include <pdal/Log.hpp>
#include <pdal/SpatialReference.hpp>

namespace pdal
{

struct BpfMuellerMatrix
{
    BpfMuellerMatrix()
    {
        static const double vals[] = {1.0, 0.0, 0.0, 0.0, 0.0, 1.0, 0.0, 0.0,
                                      0.0, 0.0, 1.0, 0.0, 0.0, 0.0, 0.0, 1.0};
        memcpy(m_vals, vals, sizeof(vals));
    }

    double m_vals[16];

    void dump()
    {
        for (size_t i = 0; i < 4; ++i)
            std::cerr << m_vals[i] << '\t';
        std::cerr << "\n";
        for (size_t i = 4; i < 8; ++i)
            std::cerr << m_vals[i] << '\t';
        std::cerr << "\n";
        for (size_t i = 8; i < 12; ++i)
            std::cerr << m_vals[i] << '\t';
        std::cerr << "\n";
        for (size_t i = 12; i < 16; ++i)
            std::cerr << m_vals[i] << '\t';
        std::cerr << "\n\n";
    }

    void apply(double& x, double& y, double& z)
    {
        double w =
            x * m_vals[12] + y * m_vals[13] + z * m_vals[14] + m_vals[15];

        x = (x * m_vals[0] + y * m_vals[1] + z * m_vals[2] + m_vals[3]) / w;
        y = (x * m_vals[4] + y * m_vals[5] + z * m_vals[6] + m_vals[7]) / w;
        z = (x * m_vals[8] + y * m_vals[9] + z * m_vals[10] + m_vals[11]) / w;
    }
};

enum class BpfFormat
{
    DimMajor,
    PointMajor,
    ByteMajor
};

std::istream& operator>>(std::istream& in, BpfFormat& format);
std::ostream& operator<<(std::ostream& in, const BpfFormat& format);

enum class BpfCoordType
{
    Cartesian,
    UTM,
    TCR,
    ENU
};

enum class BpfCompression
{
    None,
    QuickLZ,
    FastLZ,
    Zlib
};

struct BpfDimension
{
    BpfDimension()
        : m_offset(0.0), m_min((std::numeric_limits<double>::max)()),
          m_max(std::numeric_limits<double>::lowest()),
          m_id(Dimension::Id::Unknown)
    {
    }

    double m_offset;
    double m_min;
    double m_max;
    std::string m_label;
    Dimension::Id m_id;
};
typedef std::vector<BpfDimension> BpfDimensionList;

struct BpfHeader
{
    struct error : std::runtime_error
    {
        error(const std::string& err) : std::runtime_error(err) {}
    };

    BpfHeader()
        : m_version(0), m_len(176), m_numDim(0),
          m_compression(Utils::toNative(BpfCompression::None)), m_numPts(0),
          m_coordType(Utils::toNative(BpfCoordType::Cartesian)), m_coordId(0),
          m_spacing(0.0), m_startTime(0.0), m_endTime(0.0)
    {
    }

    int32_t m_version;
    std::string m_ver;
    int32_t m_len;
    int32_t m_numDim;
    BpfFormat m_pointFormat;
    uint8_t m_compression;
    int32_t m_numPts;
    int32_t m_coordType;
    int32_t m_coordId;
    float m_spacing;
    BpfMuellerMatrix m_xform;
    double m_startTime;
    double m_endTime;
    std::vector<BpfDimension> m_staticDims;
    LogPtr m_log;

    PDAL_EXPORT void setLog(const LogPtr& log)
    {
        m_log = log;
    }
    bool trySetSpatialReference(const pdal::SpatialReference&);
};

// Bundled-file descriptor used by the writer to validate the names of files
// embedded via the "bundledfile" option before they are handed to the Rust
// writer.
struct BpfUlemFile
{
    uint32_t m_len;
    std::string m_filename;
    std::vector<char> m_buf;
    std::string m_filespec;

    BpfUlemFile() : m_len(0) {}

    BpfUlemFile(uint32_t len, const std::string& filename,
                const std::string& filespec)
        : m_len(len), m_filename(filename), m_filespec(filespec)
    {
    }
};

} // namespace pdal
