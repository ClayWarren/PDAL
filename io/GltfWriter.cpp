/******************************************************************************
 * Copyright (c) 2011, Howard Butler, hobu.inc@gmail.com
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

#include "GltfWriter.hpp"

#include <pdal/PointView.hpp>
#include <pdal/util/ProgramArgs.hpp>

namespace pdal
{

static StaticPluginInfo const s_info{
    "writers.gltf",
    "Gltf Writer",
    "https://pdal.org/stages/writers.gltf.html",
    {"gltf", "glb"}};

CREATE_STATIC_STAGE(GltfWriter, s_info)

namespace
{

int rustTypeId(Dimension::Type type)
{
    using Dimension::Type;
    switch (type)
    {
    case Type::Unsigned8:
        return 0;
    case Type::Unsigned16:
        return 1;
    case Type::Unsigned32:
        return 2;
    case Type::Unsigned64:
        return 3;
    case Type::Signed8:
        return 4;
    case Type::Signed16:
        return 5;
    case Type::Signed32:
        return 6;
    case Type::Signed64:
        return 7;
    case Type::Float:
        return 8;
    case Type::Double:
    case Type::None:
        return 9;
    }
    return 9;
}

void addOption(pdal_options_t* options, const std::string& key,
               const std::string& value)
{
    pdal_options_add_str(options, key.c_str(), value.c_str());
}

void addOption(pdal_options_t* options, const std::string& key, bool value)
{
    addOption(options, key, std::string(value ? "true" : "false"));
}

void addOption(pdal_options_t* options, const std::string& key, double value)
{
    addOption(options, key, std::to_string(value));
}

void throwLastRustError(const std::string& fallback)
{
    const char* message = pdal_last_error();
    if (message && message[0])
        throw pdal_error(message);
    throw pdal_error(fallback);
}

} // namespace

GltfWriter::GltfWriter() {}

GltfWriter::~GltfWriter()
{
    if (m_rustView)
        pdal_point_view_destroy(m_rustView);
}

std::string GltfWriter::getName() const
{
    return s_info.name;
}

void GltfWriter::addArgs(ProgramArgs& args)
{
    args.add("metallic", "Metallic factor [0-1]", m_metallic);
    args.add("roughness", "Roughness factor [0-1]", m_roughness);
    args.add("red", "Base red factor [0-1]", m_red);
    args.add("green", "Base green factor [0-1]", m_green);
    args.add("blue", "Base blue factor [0-1]", m_blue);
    args.add("alpha", "Alpha factor [0-1]", m_alpha, 1.0);
    args.add("double_sided",
             "Whether the material should be applied to both sides of the "
             "faces.",
             m_doubleSided);
    args.add("colors",
             "Write color data for each vertex.  Note that most renderers "
             "will interpolate the color of each vertex across a face, so this "
             "may look odd.",
             m_colorVertices);
    args.add("normals", "Write vertex normals", m_writeNormals);
}

void GltfWriter::prepared(PointTableRef table)
{
    const bool hasNormals = table.layout()->hasDim(Dimension::Id::NormalX) &&
                            table.layout()->hasDim(Dimension::Id::NormalY) &&
                            table.layout()->hasDim(Dimension::Id::NormalZ);

    if (!hasNormals && m_writeNormals)
    {
        log()->get(LogLevel::Warning)
            << getName()
            << ": Option 'normals' is set to true, but one or more of the "
               "normal dimensions are missing. Not writing vertex normals.\n";
        m_writeNormals = false;
    }

    const bool hasColors = table.layout()->hasDim(Dimension::Id::Red) &&
                           table.layout()->hasDim(Dimension::Id::Green) &&
                           table.layout()->hasDim(Dimension::Id::Blue);

    if (!hasColors && m_colorVertices)
    {
        log()->get(LogLevel::Warning)
            << getName()
            << ": Option 'colors' is set to true, but one or more color "
               "dimensions are missing. Not writing vertex colors.\n";
        m_colorVertices = false;
    }
}

void GltfWriter::ready(PointTableRef table)
{
    if (m_rustView)
    {
        pdal_point_view_destroy(m_rustView);
        m_rustView = nullptr;
    }
    m_rustDims.clear();

    pdal_point_layout_t* rustLayout = pdal_point_layout_create();
    for (auto id : table.layout()->dims())
    {
        pdal_point_layout_register_dim(rustLayout,
                                       table.layout()->dimName(id).c_str(),
                                       rustTypeId(table.layout()->dimType(id)));
        m_rustDims.push_back(id);
    }
    m_rustView = pdal_point_view_create(rustLayout);
}

void GltfWriter::write(const PointViewPtr view)
{
    PointId offset = pdal_point_view_length(m_rustView);
    for (PointId idx = 0; idx < view->size(); ++idx)
    {
        PointId outIdx = pdal_point_view_add_point(m_rustView);
        for (Dimension::Id dim : m_rustDims)
            pdal_point_view_set_f64(m_rustView, outIdx,
                                    view->layout()->dimName(dim).c_str(),
                                    view->getFieldAs<double>(dim, idx));
    }

    TriangularMesh* mesh = view->mesh();
    if (!mesh)
    {
        log()->get(LogLevel::Warning)
            << "Attempt to write point view with no mesh. Skipping.\n";
        return;
    }

    for (size_t idx = 0; idx < mesh->size(); ++idx)
    {
        const Triangle& t = (*mesh)[idx];
        pdal_point_view_add_mesh_triangle(m_rustView, t.m_a + offset,
                                          t.m_b + offset, t.m_c + offset);
    }
}

void GltfWriter::done(PointTableRef)
{
    pdal_options_t* options = pdal_options_create();
    addOption(options, "filename", filename());
    addOption(options, "metallic", m_metallic);
    addOption(options, "roughness", m_roughness);
    addOption(options, "red", m_red);
    addOption(options, "green", m_green);
    addOption(options, "blue", m_blue);
    addOption(options, "alpha", m_alpha);
    addOption(options, "double_sided", m_doubleSided);
    addOption(options, "colors", m_colorVertices);
    addOption(options, "normals", m_writeNormals);

    pdal_writer_t* writer = pdal_writer_create_gltf(options);
    if (!writer)
    {
        pdal_options_destroy(options);
        throwLastRustError("Failed to create Rust GLTF writer.");
    }

    bool ok = pdal_writer_write_view(writer, m_rustView);
    pdal_writer_destroy(writer);
    pdal_options_destroy(options);
    if (!ok)
        throwLastRustError("Rust GLTF writer failed.");

    pdal_point_view_destroy(m_rustView);
    m_rustView = nullptr;
}

} // namespace pdal
