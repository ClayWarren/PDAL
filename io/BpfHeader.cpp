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

#include <istream>
#include <ostream>
#include <string>

#include <pdal/util/Utils.hpp>

#include "BpfHeader.hpp"

// BPF header/dimension parsing and serialization now live in the Rust BPF
// reader and writer behind the C ABI. Only two pieces remain on the C++ side:
// the BpfFormat stream operators that ProgramArgs needs to parse and print the
// writer's "format" option, and the UTM-zone lookup the writer uses to derive
// an automatic coordinate id from the spatial reference.

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

} // namespace pdal
