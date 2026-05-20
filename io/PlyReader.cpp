/******************************************************************************
 * Copyright (c) 2015, Peter J. Gadomski <pete.gadomski@gmail.com>
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

#include "PlyReader.hpp"

#include <pdal/PointView.hpp>
#include <pdal/util/Utils.hpp>

namespace pdal
{

static StaticPluginInfo const s_info{"readers.ply",
                                     "Read ply files.",
                                     "https://pdal.org/stages/reader.ply.html",
                                     {"ply"}};

CREATE_STATIC_STAGE(PlyReader, s_info)

namespace
{

void addOption(pdal_options_t* options, const std::string& key,
               const std::string& value)
{
    pdal_options_add_str(options, key.c_str(), value.c_str());
}

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

PlyReader::PlyReader() {}

PlyReader::~PlyReader()
{
    if (m_rustView)
        pdal_point_view_destroy(m_rustView);
}

std::string PlyReader::getName() const
{
    return s_info.name;
}

QuickInfo PlyReader::inspect()
{
    QuickInfo qi;

    initialize();

    uint64_t dimCount = pdal_point_view_dim_count(m_rustView);
    for (uint64_t idx = 0; idx < dimCount; ++idx)
    {
        char* rawName = pdal_point_view_dim_name(m_rustView, idx);
        if (!rawName)
            continue;
        qi.m_dimNames.push_back(Utils::tolower(rawName));
        pdal_string_free(rawName);
    }
    qi.m_pointCount = pdal_point_view_length(m_rustView);
    qi.m_valid = true;

    return qi;
}

void PlyReader::initialize()
{
    if (m_filename.empty())
        throwError("Can't read PLY file without filename.");

    if (m_rustView)
    {
        pdal_point_view_destroy(m_rustView);
        m_rustView = nullptr;
    }
    m_rustIndex = 0;
    m_meshCopied = false;

    pdal_options_t* options = pdal_options_create();
    addOption(options, "filename", m_filename);

    pdal_reader_t* reader = pdal_reader_create_ply(options);
    if (!reader)
    {
        pdal_options_destroy(options);
        throwLastRustError("Failed to create Rust PLY reader.");
    }

    m_rustView = pdal_reader_read_first(reader);
    pdal_reader_destroy(reader);
    pdal_options_destroy(options);
    if (!m_rustView)
        throwLastRustError("Rust PLY reader failed.");
}

void PlyReader::addDimensions(PointLayoutPtr layout)
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
        Dimension::Type type =
            cppType(pdal_point_view_dim_type(m_rustView, idx));
        Dimension::Id id = layout->registerOrAssignDim(name, type);
        m_dims.push_back(id);
        m_dimNames.push_back(name);
    }
}

void PlyReader::ready(PointTableRef)
{
    m_rustIndex = 0;
    m_meshCopied = false;
}

void PlyReader::copyPoint(PointRef& point, PointId rustIndex)
{
    for (size_t dimIdx = 0; dimIdx < m_dims.size(); ++dimIdx)
        point.setField(m_dims[dimIdx],
                       pdal_point_view_get_f64(m_rustView, rustIndex,
                                               m_dimNames[dimIdx].c_str()));
}

void PlyReader::copyMesh(PointViewPtr view)
{
    if (m_meshCopied)
        return;

    uint64_t triangleCount = pdal_point_view_mesh_triangle_count(m_rustView);
    if (!triangleCount)
    {
        m_meshCopied = true;
        return;
    }

    TriangularMesh* mesh = view->createMesh("ply");
    if (!mesh)
        throwError("Failed to create mesh");

    for (uint64_t idx = 0; idx < triangleCount; ++idx)
    {
        uint64_t a = 0;
        uint64_t b = 0;
        uint64_t c = 0;
        if (!pdal_point_view_mesh_triangle(m_rustView, idx, &a, &b, &c))
            throwError("Rust PLY reader failed to return a mesh triangle.");
        mesh->add(a, b, c);
    }
    m_meshCopied = true;
}

bool PlyReader::processOne(PointRef& point)
{
    if (m_rustIndex >= pdal_point_view_length(m_rustView))
        return false;

    copyPoint(point, m_rustIndex);
    ++m_rustIndex;
    return true;
}

point_count_t PlyReader::read(PointViewPtr view, point_count_t num)
{
    point_count_t cnt = 0;
    while (cnt < num && m_rustIndex < pdal_point_view_length(m_rustView))
    {
        PointRef point(*view, view->size());
        copyPoint(point, m_rustIndex);
        ++m_rustIndex;
        ++cnt;
    }
    copyMesh(view);
    return cnt;
}

void PlyReader::done(PointTableRef) {}

} // namespace pdal
