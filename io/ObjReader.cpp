/******************************************************************************
 * Copyright (c) 2020, Hobu Inc., info@hobu.co
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
 *     * Neither the name of Hobu, Inc. nor the
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

#include "ObjReader.hpp"

#include <pdal/PointView.hpp>

namespace pdal
{

static StaticPluginInfo const s_info{"readers.obj",
                                     "Obj Reader",
                                     "https://pdal.org/stages/readers.obj.html",
                                     {"obj"}};

CREATE_STATIC_STAGE(ObjReader, s_info)

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

std::string ObjReader::getName() const
{
    return s_info.name;
}

ObjReader::ObjReader() {}

ObjReader::~ObjReader()
{
    if (m_rustView)
        pdal_point_view_destroy(m_rustView);
}

QuickInfo ObjReader::inspect()
{
    QuickInfo qi;

    initialize();

    uint64_t dimCount = pdal_point_view_dim_count(m_rustView);
    for (uint64_t idx = 0; idx < dimCount; ++idx)
    {
        char* rawName = pdal_point_view_dim_name(m_rustView, idx);
        if (!rawName)
            continue;
        qi.m_dimNames.push_back(rawName);
        pdal_string_free(rawName);
    }
    qi.m_pointCount = pdal_point_view_length(m_rustView);
    qi.m_valid = true;

    return qi;
}

void ObjReader::initialize()
{
    if (m_filename.empty())
        throwError("Can't read OBJ file without filename.");

    if (m_rustView)
    {
        pdal_point_view_destroy(m_rustView);
        m_rustView = nullptr;
    }
    m_copied = false;

    pdal_options_t* options = pdal_options_create();
    addOption(options, "filename", m_filename);

    pdal_reader_t* reader = pdal_reader_create_obj(options);
    if (!reader)
    {
        pdal_options_destroy(options);
        throwLastRustError("Failed to create Rust OBJ reader.");
    }

    m_rustView = pdal_reader_read_first(reader);
    pdal_reader_destroy(reader);
    pdal_options_destroy(options);
    if (!m_rustView)
        throwLastRustError("Rust OBJ reader failed.");
}

void ObjReader::addDimensions(PointLayoutPtr layout)
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

void ObjReader::ready(PointTableRef) {}

point_count_t ObjReader::read(PointViewPtr view, point_count_t)
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

    TriangularMesh* mesh = view->createMesh("obj");
    if (!mesh)
        throwError("Failed to create mesh");

    uint64_t triangleCount = pdal_point_view_mesh_triangle_count(m_rustView);
    for (uint64_t idx = 0; idx < triangleCount; ++idx)
    {
        uint64_t a = 0;
        uint64_t b = 0;
        uint64_t c = 0;
        if (!pdal_point_view_mesh_triangle(m_rustView, idx, &a, &b, &c))
            throwError("Rust OBJ reader failed to return a mesh triangle.");
        mesh->add(a, b, c);
    }

    m_copied = true;
    return count;
}

void ObjReader::done(PointTableRef) {}

} // namespace pdal
