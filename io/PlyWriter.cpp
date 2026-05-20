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

#include "PlyWriter.hpp"

#include <limits>
#include <sstream>

#include <pdal/util/OStream.hpp>
#include <pdal/util/ProgramArgs.hpp>

#include <rust/pdal-capi/include/pdal_capi.h>

namespace pdal
{

static StaticPluginInfo const s_info{"writers.ply",
                                     "ply writer",
                                     "https://pdal.org/stages/writers.ply.html",
                                     {"ply"}};

CREATE_STATIC_STAGE(PlyWriter, s_info)

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

std::string boolString(bool value)
{
    return value ? "true" : "false";
}

std::string storageMode(PlyWriter::Format format)
{
    switch (format)
    {
    case PlyWriter::Format::Ascii:
        return "ascii";
    case PlyWriter::Format::BinaryLe:
        return "binary_little_endian";
    case PlyWriter::Format::BinaryBe:
        return "binary_big_endian";
    }
    return "ascii";
}

std::string rustDimSpecType(Dimension::Type type)
{
    using Dimension::Type;
    switch (type)
    {
    case Type::Unsigned8:
        return "uint8_t";
    case Type::Unsigned16:
        return "uint16_t";
    case Type::Unsigned32:
        return "uint32_t";
    case Type::Unsigned64:
        return "uint64_t";
    case Type::Signed8:
        return "int8_t";
    case Type::Signed16:
        return "int16_t";
    case Type::Signed32:
        return "int32_t";
    case Type::Signed64:
        return "int64_t";
    case Type::Float:
        return "float";
    case Type::Double:
    case Type::None:
        return "double";
    }
    return "double";
}

std::string rustDimSpecs(const StringList& names, const DimTypeList& dims)
{
    std::ostringstream out;
    for (size_t idx = 0; idx < dims.size(); ++idx)
    {
        if (idx)
            out << ',';
        out << names[idx] << '=' << rustDimSpecType(dims[idx].m_type);
    }
    return out.str();
}

std::string rustStorageName(const PointLayoutPtr& layout, Dimension::Id id,
                            const std::string& plyName)
{
    switch (id)
    {
    case Dimension::Id::X:
    case Dimension::Id::Y:
    case Dimension::Id::Z:
        return layout->dimName(id);
    default:
        return plyName;
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

std::string PlyWriter::getName() const
{
    return s_info.name;
}

PlyWriter::PlyWriter() {}

void PlyWriter::addArgs(ProgramArgs& args)
{
    args.add("storage_mode", "PLY Storage Mode", m_format, Format::Ascii);
    args.add("dims", "Dimension names", m_dimNames);
    args.add("faces", "Write faces", m_faces);
    args.add("sized_types",
             "Write types as size-explicit strings (e.g. 'uint32')",
             m_sizedTypes, true);
    m_precisionArg = &args.add("precision", "Output precision", m_precision, 3);
}

void PlyWriter::prepared(PointTableRef table)
{
    FlexWriter::validateFilename(table);

    PointLayoutPtr layout = table.layout();

    if (m_precisionArg->set() && m_format != Format::Ascii)
        throwError("Option 'precision' can only be set of the 'storage_mode' "
                   "is ascii.");

    m_dims.clear();
    if (m_dimNames.size())
    {
        for (auto& dimspec : m_dimNames)
        {
            Dimension::Type type;
            StringList parts = Utils::split2(dimspec, '=');
            std::string name = parts[0];
            Utils::trim(name);
            dimspec = Utils::tolower(name);
            auto id = layout->findDim(name);
            if (id == Dimension::Id::Unknown)
                throwError("Unknown dimension '" + name +
                           "' in provided "
                           "dimension list.");
            if (parts.size() == 1)
                type = layout->dimType(id);
            else if (parts.size() == 2)
            {
                Utils::trim(parts[1]);
                type = Dimension::type(parts[1]);
            }
            else
                throwError("Invalid format for dimension specification for "
                           "dimension '" +
                           name +
                           "'. "
                           "Must be 'name[=type]'.");
            m_dims.emplace_back(id, type);
        }
    }
    else
    {
        m_dims = layout->dimTypes();
        for (const auto& dim : m_dims)
            m_dimNames.push_back(Utils::tolower(layout->dimName(dim.m_id)));
    }
}

std::string PlyWriter::getType(Dimension::Type type) const
{
    static std::map<Dimension::Type, std::string> sizedTypes = {
        {Dimension::Type::Signed8, "int8"},
        {Dimension::Type::Unsigned8, "uint8"},
        {Dimension::Type::Signed16, "int16"},
        {Dimension::Type::Unsigned16, "uint16"},
        {Dimension::Type::Signed32, "int32"},
        {Dimension::Type::Unsigned32, "uint32"},
        {Dimension::Type::Float, "float32"},
        {Dimension::Type::Double, "float64"}};
    static std::map<Dimension::Type, std::string> types = {
        {Dimension::Type::Signed8, "char"},
        {Dimension::Type::Unsigned8, "uchar"},
        {Dimension::Type::Signed16, "short"},
        {Dimension::Type::Unsigned16, "ushort"},
        {Dimension::Type::Signed32, "int"},
        {Dimension::Type::Unsigned32, "uint"},
        {Dimension::Type::Float, "float"},
        {Dimension::Type::Double, "double"}};

    try
    {
        return m_sizedTypes ? sizedTypes.at(type) : types.at(type);
    }
    catch (std::out_of_range&)
    {
        throwError("Can't write dimension of type '" +
                   Dimension::interpretationName(type) + "'.");
    }
    return "";
}

void PlyWriter::writeHeader(PointLayoutPtr layout, point_count_t pointCount,
                            point_count_t faceCount) const
{
    *m_stream << "ply" << '\n';
    *m_stream << "format " << m_format << " 1.0" << '\n';
    *m_stream << "comment Generated by PDAL" << '\n';
    *m_stream << "element vertex " << pointCount << '\n';

    assert(m_dimNames.size() == m_dims.size());
    auto ni = m_dimNames.begin();
    for (auto dim : m_dims)
    {
        std::string name = *ni++;
        std::string typeString = getType(dim.m_type);
        *m_stream << "property " << typeString << " " << name << '\n';
    }
    if (m_faces)
    {
        *m_stream << "element face " << faceCount << '\n';
        if (m_sizedTypes)
            *m_stream << "property list uint8 uint32 vertex_indices";
        else
            *m_stream << "property list uchar uint vertex_indices";
        *m_stream << '\n';
    }
    *m_stream << "end_header" << '\n';
}

void PlyWriter::readyTable(PointTableRef table)
{
    m_layout = table.layout();
}

void PlyWriter::doneTable(PointTableRef table)
{
    m_layout = nullptr;
}

void PlyWriter::readyFile(const std::string& filename,
                          const SpatialReference& srs)
{
    m_curFilename = filename;
    Utils::writeProgress(m_progressFd, "READYFILE", filename);
}

void PlyWriter::writeView(const PointViewPtr data)
{
    m_views.push_back(data);
}

template <typename T>
void writeTextVal(std::ostream& out, PointRef& point, Dimension::Id dim)
{
    T t = point.getFieldAs<T>(dim);
    out << t;
}

// Writing int/uint can write "char" type instead of numeric values.
// Two specializations is easier than the SFINAE mess.
template <>
void writeTextVal<int8_t>(std::ostream& out, PointRef& point, Dimension::Id dim)
{
    int i = point.getFieldAs<int8_t>(dim);
    out << i;
}

template <>
void writeTextVal<uint8_t>(std::ostream& out, PointRef& point,
                           Dimension::Id dim)
{
    uint32_t i = point.getFieldAs<uint8_t>(dim);
    out << i;
}

void PlyWriter::writeValue(PointRef& point, Dimension::Id dim,
                           Dimension::Type type)
{
    if (m_format == Format::Ascii)
    {
        if (Dimension::base(type) == Dimension::BaseType::Floating)
        {
            if (m_precisionArg->set())
            {
                *m_stream << std::fixed;
                m_stream->precision(m_precision);
            }

            if (type == Dimension::Type::Float)
                writeTextVal<float>(*m_stream, point, dim);
            else
                writeTextVal<double>(*m_stream, point, dim);

            if (m_precisionArg->set())
                m_stream->unsetf(std::ios_base::fixed);
        }
        else
        {
            switch (type)
            {
            case Dimension::Type::Unsigned8:
                writeTextVal<uint8_t>(*m_stream, point, dim);
                break;
            case Dimension::Type::Unsigned16:
                writeTextVal<uint16_t>(*m_stream, point, dim);
                break;
            case Dimension::Type::Unsigned32:
                writeTextVal<uint32_t>(*m_stream, point, dim);
                break;
            case Dimension::Type::Unsigned64:
                writeTextVal<uint64_t>(*m_stream, point, dim);
                break;
            case Dimension::Type::Signed8:
                writeTextVal<int8_t>(*m_stream, point, dim);
                break;
            case Dimension::Type::Signed16:
                writeTextVal<int16_t>(*m_stream, point, dim);
                break;
            case Dimension::Type::Signed32:
                writeTextVal<int32_t>(*m_stream, point, dim);
                break;
            case Dimension::Type::Signed64:
                writeTextVal<int64_t>(*m_stream, point, dim);
                break;
            default:
                throwError("Internal error: invalid type found writing "
                           "output.");
            }
        }
    }
    else if (m_format == Format::BinaryLe)
    {
        OLeStream out(m_stream);
        Everything e;
        point.getField((char*)&e, dim, type);
        Utils::insertDim(out, type, e);
    }
    else if (m_format == Format::BinaryBe)
    {
        OBeStream out(m_stream);
        Everything e;
        point.getField((char*)&e, dim, type);
        Utils::insertDim(out, type, e);
    }
}

void PlyWriter::writePoint(PointRef& point, PointLayoutPtr layout)
{
    for (auto it = m_dims.begin(); it != m_dims.end();)
    {
        writeValue(point, it->m_id, it->m_type);
        ++it;
        if (m_format == Format::Ascii && it != m_dims.end())
            *m_stream << " ";
    }
    if (m_format == Format::Ascii)
        *m_stream << '\n';
}

void PlyWriter::writeTriangle(const Triangle& t, size_t offset)
{
    if (m_format == Format::Ascii)
    {
        *m_stream << "3 " << (t.m_a + offset) << " " << (t.m_b + offset) << " "
                  << (t.m_c + offset) << '\n';
    }
    else if (m_format == Format::BinaryLe)
    {
        OLeStream out(m_stream);
        unsigned char count = 3;
        uint32_t a = (uint32_t)(t.m_a + offset);
        uint32_t b = (uint32_t)(t.m_b + offset);
        uint32_t c = (uint32_t)(t.m_c + offset);
        out << count << a << b << c;
    }
    else if (m_format == Format::BinaryBe)
    {
        OBeStream out(m_stream);
        unsigned char count = 3;
        uint32_t a = (uint32_t)(t.m_a + offset);
        uint32_t b = (uint32_t)(t.m_b + offset);
        uint32_t c = (uint32_t)(t.m_c + offset);
        out << count << a << b << c;
    }
}

// Deferring write until this time allows both points and faces from multiple
// point views to be written.
void PlyWriter::doneFile()
{
    point_count_t pointCount = 0;
    for (auto& v : m_views)
        pointCount += v->size();

    if (pointCount > (std::numeric_limits<uint32_t>::max)())
        throwError("Can't write PLY file.  Only " +
                   std::to_string((std::numeric_limits<uint32_t>::max)()) +
                   " points supported.");

    pdal_point_layout_t* rustLayout = pdal_point_layout_create();
    assert(m_dimNames.size() == m_dims.size());
    auto name = m_dimNames.begin();
    for (const auto& dim : m_dims)
    {
        Dimension::Type storedType = m_layout->dimType(dim.m_id);
        std::string storageName = rustStorageName(m_layout, dim.m_id, *name);
        pdal_point_layout_register_dim(rustLayout, storageName.c_str(),
                                       rustTypeId(storedType));
        ++name;
    }

    pdal_point_view_t* rustView = pdal_point_view_create(rustLayout);
    for (const auto& v : m_views)
    {
        PointRef point(*v, 0);
        for (PointId idx = 0; idx < v->size(); ++idx)
        {
            point.setPointId(idx);
            PointId outIdx = pdal_point_view_add_point(rustView);
            auto dimName = m_dimNames.begin();
            for (const auto& dim : m_dims)
            {
                std::string storageName =
                    rustStorageName(m_layout, dim.m_id, *dimName);
                pdal_point_view_set_f64(rustView, outIdx, storageName.c_str(),
                                        point.getFieldAs<double>(dim.m_id));
                ++dimName;
            }
        }
    }
    if (m_faces)
    {
        PointId offset = 0;
        for (const auto& v : m_views)
        {
            TriangularMesh* mesh = v->mesh();
            if (mesh)
            {
                for (size_t id = 0; id < mesh->size(); ++id)
                {
                    const Triangle& t = (*mesh)[id];
                    pdal_point_view_add_mesh_triangle(rustView, t.m_a + offset,
                                                      t.m_b + offset,
                                                      t.m_c + offset);
                }
            }
            offset += v->size();
        }
    }

    pdal_options_t* options = pdal_options_create();
    addOption(options, "filename", m_curFilename);
    addOption(options, "storage_mode", storageMode(m_format));
    addOption(options, "faces", boolString(m_faces));
    addOption(options, "sized_types", boolString(m_sizedTypes));
    addOption(options, "dims", rustDimSpecs(m_dimNames, m_dims));
    if (m_precisionArg->set())
        addOption(options, "precision", std::to_string(m_precision));

    pdal_writer_t* writer = pdal_writer_create_ply(options);
    if (!writer)
    {
        pdal_point_view_destroy(rustView);
        pdal_options_destroy(options);
        throwLastRustError("Failed to create Rust PLY writer.");
    }

    bool ok = pdal_writer_write_view(writer, rustView);
    pdal_writer_destroy(writer);
    pdal_point_view_destroy(rustView);
    pdal_options_destroy(options);
    if (!ok)
        throwLastRustError("Rust PLY writer failed.");

    m_views.clear();

    Utils::writeProgress(m_progressFd, "DONEFILE", m_curFilename);
    getMetadata().addList("filename", m_curFilename);
    m_curFilename.clear();
}

} // namespace pdal
