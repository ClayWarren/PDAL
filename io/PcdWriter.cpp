/******************************************************************************
 * Copyright (c) 2019, Bradley J Chambers (brad.chambers@gmail.com)
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

#include "PcdWriter.hpp"
#include "PcdHeader.hpp"

#include <pdal/PDALUtils.hpp>
#include <pdal/util/ProgramArgs.hpp>

namespace pdal
{

using namespace Dimension;

static StaticPluginInfo const s_info{
    "writers.pcd",
    "Write data in the Point Cloud Library (PCL) format.",
    "https://pdal.org/stages/writers.pcd.html",
    {"pcd"}};

CREATE_STATIC_STAGE(PcdWriter, s_info)

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

void addOption(pdal_options_t* options, const std::string& key, uint32_t value)
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

std::string PcdWriter::getName() const
{
    return s_info.name;
}

PcdWriter::PcdWriter() {}

PcdWriter::~PcdWriter()
{
    if (m_rustView)
        pdal_point_view_destroy(m_rustView);
}

void PcdWriter::addArgs(ProgramArgs& args)
{
    args.add("compression",
             "Level of PCD compression to use (ascii, binary, compressed)",
             m_compression_string, "ascii");
    args.add("keep_unspecified", "Write all dimensions", m_writeAllDims, true);
    args.add("order", "Dimension order", m_dimOrder);
    args.add("precision", "ASCII precision", m_precision,
             static_cast<uint32_t>(2));
}

PcdWriter::DimSpec PcdWriter::extractDim(std::string dim, PointTableRef table)
{
    Utils::trim(dim);

    uint32_t precision(m_precision);
    PcdField field;
    field.m_count = 1;
    StringList s = Utils::split(dim, '=');
    if (s.size() == 1)
    {
        precision = m_precision;
        Id id = table.layout()->findDim(s[0]);
        if (id == Id::X || id == Id::Y || id == Id::Z)
            field.m_size = 4;
        else
            field.m_size = 8;
        field.m_type = PcdFieldType::F;
    }
    else if (s.size() == 2)
    {
        try
        {
            StringList t = Utils::split(s[1], ':');

            if (t[0] == "Unsigned8")
            {
                field.m_type = PcdFieldType::U;
                field.m_size = 1;
            }
            else if (t[0] == "Unsigned16")
            {
                field.m_type = PcdFieldType::U;
                field.m_size = 2;
            }
            else if (t[0] == "Unsigned32")
            {
                field.m_type = PcdFieldType::U;
                field.m_size = 4;
            }
            else if (t[0] == "Unsigned64")
            {
                field.m_type = PcdFieldType::U;
                field.m_size = 8;
            }
            else if (t[0] == "Signed8")
            {
                field.m_type = PcdFieldType::I;
                field.m_size = 1;
            }
            else if (t[0] == "Signed16")
            {
                field.m_type = PcdFieldType::I;
                field.m_size = 2;
            }
            else if (t[0] == "Signed32")
            {
                field.m_type = PcdFieldType::I;
                field.m_size = 4;
            }
            else if (t[0] == "Signed64")
            {
                field.m_type = PcdFieldType::I;
                field.m_size = 8;
            }
            else if (t[0] == "Float")
            {
                field.m_type = PcdFieldType::F;
                field.m_size = 4;
            }
            else if (t[0] == "Double")
            {
                field.m_type = PcdFieldType::F;
                field.m_size = 8;
            }
            else
            {
                field.m_type = PcdFieldType::unknown;
            }

            if (t.size() == 2)
            {
                size_t pos;
                int i = std::stoi(t[1], &pos);
                if (i < 0 || pos != t[1].size())
                    throw pdal_error("Dummy"); // Throw to be caught below.
                precision = static_cast<uint32_t>(i);
            }
        }
        catch (...)
        {
            throwError("Can't convert dimension precision for '" + dim + "'.");
        }
    }
    else
        throwError("Invalid dimension specification '" + dim + "'.");
    Id d = table.layout()->findDim(s[0]);
    if (d == Id::Unknown)
        throwError("Dimension not found with name '" + dim + "'.");

    field.m_label = table.layout()->dimName(d);
    field.m_id = d;

    return DimSpec{field, precision};
}

bool PcdWriter::findDim(Id id, DimSpec& ds)
{
    auto it =
        std::find_if(m_dims.begin(), m_dims.end(), [id](const DimSpec& tds)
                     { return tds.m_field.m_id == id; });
    if (it == m_dims.end())
        return false;
    ds = *it;
    return true;
}

void PcdWriter::ready(PointTableRef table)
{
    if (m_rustView)
    {
        pdal_point_view_destroy(m_rustView);
        m_rustView = nullptr;
    }
    m_dims.clear();
    m_rustDims.clear();

    PcdField field;
    field.m_label = table.layout()->dimName(Id::X);
    field.m_id = Id::X;
    field.m_size = 4;
    field.m_type = PcdFieldType::F;
    field.m_count = 1;
    m_xDim = DimSpec{field, m_precision};
    field.m_label = table.layout()->dimName(Id::Y);
    field.m_id = Id::Y;
    m_yDim = {field, m_precision};
    field.m_label = table.layout()->dimName(Id::Z);
    field.m_id = Id::Z;
    m_zDim = {field, m_precision};

    // Find the dimensions listed and put them on the id list.
    StringList dimNames = Utils::split2(m_dimOrder, ',');
    for (const std::string& dim : dimNames)
    {
        const DimSpec& spec = extractDim(dim, table);
        if (spec.m_field.m_id == Id::X)
            m_xDim = spec;
        else if (spec.m_field.m_id == Id::Y)
            m_yDim = spec;
        else if (spec.m_field.m_id == Id::Z)
            m_zDim = spec;
        m_dims.push_back(spec);
    }

    if (m_dimOrder.empty() || m_writeAllDims)
    {
        IdList all = table.layout()->dims();
        for (auto id : all)
        {
            PcdField field;
            field.m_label = table.layout()->dimName(id);
            field.m_id = id;
            if (id == Id::X || id == Id::Y || id == Id::Z)
                field.m_size = 4;
            else
                field.m_size = 8;
            field.m_type = PcdFieldType::F;
            field.m_count = 1;
            DimSpec ds{field, m_precision};
            if (!findDim(id, ds))
                m_dims.push_back(ds);
        }
    }

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

void PcdWriter::write(const PointViewPtr view)
{
    for (PointId idx = 0; idx < view->size(); ++idx)
    {
        PointId outIdx = pdal_point_view_add_point(m_rustView);
        for (Dimension::Id dim : m_rustDims)
            pdal_point_view_set_f64(m_rustView, outIdx,
                                    view->layout()->dimName(dim).c_str(),
                                    view->getFieldAs<double>(dim, idx));
    }
}

void PcdWriter::done(PointTableRef table)
{
    pdal_options_t* options = pdal_options_create();
    addOption(options, "filename", filename());
    addOption(options, "compression", m_compression_string);
    addOption(options, "keep_unspecified", m_writeAllDims);
    addOption(options, "order", m_dimOrder);
    addOption(options, "precision", m_precision);

    pdal_writer_t* writer = pdal_writer_create_pcd(options);
    if (!writer)
    {
        pdal_options_destroy(options);
        throwLastRustError("Failed to create Rust PCD writer.");
    }

    bool ok = pdal_writer_write_view(writer, m_rustView);
    pdal_writer_destroy(writer);
    pdal_options_destroy(options);
    if (!ok)
        throwLastRustError("Rust PCD writer failed.");

    pdal_point_view_destroy(m_rustView);
    m_rustView = nullptr;
    getMetadata().addList("filename", filename());
}

} // namespace pdal
