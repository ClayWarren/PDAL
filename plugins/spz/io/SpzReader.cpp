/******************************************************************************
 * Copyright (c) 2025, Isaac Bell (isaac@hobu.co)
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
 *       the documentation and/or other materials provided with the
 *       distribution.
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

#include "SpzReader.hpp"

#include <pdal/PDALUtils.hpp>
#include <pdal/PointView.hpp>

namespace pdal
{

static StaticPluginInfo const s_info{"readers.spz",
                                     "SPZ Reader",
                                     "https://pdal.org/stages/readers.spz.html",
                                     {"spz"}};

CREATE_SHARED_STAGE(SpzReader, s_info)

namespace
{

Dimension::Type cppType(int type)
{
    using Dimension::Type;
    switch (type)
    {
    case 0:
        return Type::Unsigned8;
    case 1:
        return Type::Unsigned16;
    case 2:
        return Type::Unsigned32;
    case 3:
        return Type::Unsigned64;
    case 4:
        return Type::Signed8;
    case 5:
        return Type::Signed16;
    case 6:
        return Type::Signed32;
    case 7:
        return Type::Signed64;
    case 8:
        return Type::Float;
    case 9:
    default:
        return Type::Double;
    }
}

void throwLastRustError(const std::string& fallback)
{
    const char* message = pdal_last_error();
    if (message && message[0])
        throw pdal_error(message);
    throw pdal_error(fallback);
}

} // namespace

SpzReader::~SpzReader()
{
    if (m_rustView)
        pdal_point_view_destroy(m_rustView);
}

std::string SpzReader::getName() const
{
    return s_info.name;
}

void SpzReader::addArgs(ProgramArgs& args) {}

void SpzReader::initialize()
{
    if (m_rustView)
    {
        pdal_point_view_destroy(m_rustView);
        m_rustView = nullptr;
    }
    m_copied = false;

    pdal_options_t* options = pdal_options_create();
    pdal_options_add_str(options, "filename", m_filename.c_str());

    pdal_reader_t* reader = pdal_reader_create_spz(options);
    if (!reader)
    {
        pdal_options_destroy(options);
        throwLastRustError("Failed to create Rust SPZ reader.");
    }

    m_rustView = pdal_reader_read_first(reader);
    pdal_reader_destroy(reader);
    pdal_options_destroy(options);
    if (!m_rustView)
        throwLastRustError("Rust SPZ reader failed.");
}

void SpzReader::addDimensions(PointLayoutPtr layout)
{
    m_dims.clear();
    m_dimNames.clear();
    uint64_t dimCount = pdal_point_view_dim_count(m_rustView);
    for (uint64_t idx = 0; idx < dimCount; ++idx)
    {
        char* rawName = pdal_point_view_dim_name(m_rustView, idx);
        if (!rawName)
            continue;
        std::string name(rawName);
        pdal_string_free(rawName);
        Dimension::Id id = layout->registerOrAssignDim(
            name, cppType(pdal_point_view_dim_type(m_rustView, idx)));
        m_dims.push_back(id);
        m_dimNames.push_back(name);
    }
}

void SpzReader::ready(PointTableRef table)
{
    table.metadata().add("coordinate_orientation", "RUB");
}

point_count_t SpzReader::read(PointViewPtr view, point_count_t)
{
    if (m_copied)
        return 0;

    point_count_t count = pdal_point_view_length(m_rustView);
    for (PointId idx = 0; idx < count; ++idx)
    {
        PointRef point(*view, view->size());
        for (size_t dimIdx = 0; dimIdx < m_dims.size(); ++dimIdx)
            point.setField(m_dims[dimIdx],
                           pdal_point_view_get_f64(m_rustView, idx,
                                                   m_dimNames[dimIdx].c_str()));
    }

    m_copied = true;
    return count;
}

void SpzReader::done(PointTableRef) {}

} // namespace pdal
