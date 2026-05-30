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

#include <iostream>

#include <pdal/util/IStream.hpp>

#include "BpfHeader.hpp"

namespace pdal
{

std::istream& operator>>(std::istream& in, BpfFormat& format)
{
    std::string s;

    in >> s;
    s = Utils::toupper(s);
    if (s == "POINT")
        format = BpfFormat::PointMajor;
    else if (s == "BYTE")
        format = BpfFormat::ByteMajor;
    else if ((s == "DIM") || (s == "DIMENSION"))
        format = BpfFormat::DimMajor;
    else
        in.setstate(std::ios::failbit);
    return in;
}

std::ostream& operator<<(std::ostream& out, const BpfFormat& format)
{
    switch (format)
    {
    case BpfFormat::PointMajor:
        out << "Point";
    case BpfFormat::ByteMajor:
        out << "Byte";
    case BpfFormat::DimMajor:
        out << "Dimension";
    }
    return out;
}

ILeStream& operator>>(ILeStream& stream, BpfMuellerMatrix& m)
{
    for (size_t i = 0; i < (sizeof(m.m_vals) / sizeof(m.m_vals[0])); ++i)
        stream >> m.m_vals[i];
    return stream;
}

bool BpfHeader::read(ILeStream& stream)
{
    IStreamMarker mark(stream);
    if (!readV3(stream))
    {
        mark.rewind();
        if (!readV1(stream))
        {
            if (m_version < 1 || m_version > 3)
                m_log->get(LogLevel::Error)
                    << "Unsupported BPF version = " << m_version << ".\n";
            else
                m_log->get(LogLevel::Error) << "Couldn't read BPF header.\n";
            return false;
        }
    }
    return true;
}

bool BpfHeader::readV3(ILeStream& stream)
{
    m_log->get(LogLevel::Debug) << "BPF: Reading V3\n";

    uint8_t dummyChar;
    uint8_t interleave;
    std::string magic;

    stream.get(magic, 4);
    if (magic != "BPF!")
        return false;

    stream.get(m_ver, 4);
    Utils::fromString(m_ver, m_version);

    uint8_t numDim;
    stream >> m_len >> numDim >> interleave >> m_compression >> dummyChar >>
        m_numPts >> m_coordType >> m_coordId >> m_spacing >> m_xform >>
        m_startTime >> m_endTime;
    m_numDim = (int32_t)numDim;

    switch (interleave)
    {
    case 0:
        m_pointFormat = BpfFormat::DimMajor;
        break;
    case 1:
        m_pointFormat = BpfFormat::PointMajor;
        break;
    case 2:
        m_pointFormat = BpfFormat::ByteMajor;
        break;
    default:
        throw error("Invalid BPF file: unknown interleave type.");
    }
    return (bool)stream;
}

bool BpfHeader::trySetSpatialReference(const SpatialReference& srs)
{
    m_log->get(LogLevel::Debug)
        << "Attempting to set coordinate system UTM zone \n";

    int zone = srs.getUTMZone();
    if (zone)
    {
        m_coordId = zone;
        return true;
    }
    return false;
}

bool BpfHeader::readV1(ILeStream& stream)
{
    m_log->get(LogLevel::Debug) << "BPF: Reading V1\n";

    stream >> m_len;
    stream >> m_version;

    stream >> m_numPts >> m_numDim >> m_coordType >> m_coordId >> m_spacing;

    if (m_version == 1)
        m_pointFormat = BpfFormat::DimMajor;
    else if (m_version == 2)
        m_pointFormat = BpfFormat::PointMajor;
    else
        return false;

    // Dimensions should include X, Y, and Z
    m_numDim += 3;

    BpfDimension xDim;
    BpfDimension yDim;
    BpfDimension zDim;

    xDim.m_label = "X";
    yDim.m_label = "Y";
    zDim.m_label = "Z";

    stream >> xDim.m_offset >> yDim.m_offset >> zDim.m_offset;
    stream >> xDim.m_min >> xDim.m_max;
    stream >> yDim.m_min >> yDim.m_max;
    stream >> zDim.m_min >> zDim.m_max;

    m_staticDims.resize(3);
    m_staticDims[0] = xDim;
    m_staticDims[1] = yDim;
    m_staticDims[2] = zDim;
    return (bool)stream;
}

bool BpfHeader::readDimensions(ILeStream& stream, BpfDimensionList& dims,
                               bool fixNames)
{
    size_t staticCnt = m_staticDims.size();

    dims.resize(m_numDim);

    if (static_cast<std::size_t>(m_numDim) < staticCnt)
    {
        m_log->get(LogLevel::Error) << "BPF dimension range looks bad.\n";
        m_log->get(LogLevel::Error)
            << "BPF: num dims: " << m_numDim << "\n"
            << "BPF: static count: " << staticCnt << "\n";

        m_log->get(LogLevel::Error) << "Dims:\n";
        for (const auto& d : dims)
            m_log->get(LogLevel::Error) << "\t" << d.m_label << "\n";

        m_log->get(LogLevel::Error) << "Static:\n";
        for (const auto& d : m_staticDims)
            m_log->get(LogLevel::Error) << "\t" << d.m_label << "\n";
    }

    for (size_t d = 0; d < staticCnt; d++)
        dims.at(d) = m_staticDims[d];
    if (!BpfDimension::read(stream, dims, staticCnt))
        return false;

    // Verify that we have an X, Y and Z, so that we don't have to worry
    // about it later.
    bool x = false;
    bool y = false;
    bool z = false;
    for (auto& d : dims)
    {
        if (d.m_label == "X")
            x = true;
        if (d.m_label == "Y")
            y = true;
        if (d.m_label == "Z")
            z = true;

        if (fixNames)
            d.m_label = Dimension::fixName(d.m_label);
    }
    if (!x || !y || !z)
        throw error("BPF file missing at least one of X, Y or Z dimensions.");
    return true;
}

bool BpfDimension::read(ILeStream& stream, BpfDimensionList& dims, size_t start)
{
    for (size_t d = start; d < dims.size(); ++d)
        stream >> dims[d].m_offset;
    for (size_t d = start; d < dims.size(); ++d)
        stream >> dims[d].m_min;
    for (size_t d = start; d < dims.size(); ++d)
        stream >> dims[d].m_max;
    for (size_t d = start; d < dims.size(); ++d)
        stream.get(dims[d].m_label, 32);
    return (bool)stream;
}

} // namespace pdal
