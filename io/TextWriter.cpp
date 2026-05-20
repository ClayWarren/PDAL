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

#include "TextWriter.hpp"

#include <pdal/PDALUtils.hpp>
#include <pdal/PointView.hpp>
#include <pdal/pdal_export.hpp>
#include <pdal/util/Algorithm.hpp>
#include <pdal/util/ProgramArgs.hpp>

namespace pdal
{

static StaticPluginInfo const s_info{
    "writers.text",
    "Text Writer",
    "https://pdal.org/stages/writers.text.html",
    {"csv", "txt", "json", "xyz", ""}};

CREATE_STATIC_STAGE(TextWriter, s_info)

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

void addOption(pdal_options_t* options, const std::string& key, int value)
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

TextWriter::~TextWriter()
{
    if (m_rustView)
        pdal_point_view_destroy(m_rustView);
}

std::string TextWriter::getName() const
{
    return s_info.name;
}

std::istream& operator>>(std::istream& in, TextWriter::OutputType& type)
{
    std::string s;
    in >> s;
    s = Utils::toupper(s);
    if (s == "CSV")
        type = TextWriter::OutputType::CSV;
    else if (s == "GEOJSON")
        type = TextWriter::OutputType::GEOJSON;
    else
        in.setstate(std::ios_base::failbit);
    return in;
}

std::ostream& operator<<(std::ostream& out, const TextWriter::OutputType& type)
{
    if (type == TextWriter::OutputType::CSV)
        out << "CSV";
    else if (type == TextWriter::OutputType::GEOJSON)
        out << "GEOJSON";
    return out;
}

void TextWriter::addArgs(ProgramArgs& args)
{
    args.add("format", "Output format", m_outputType, OutputType::CSV);
    args.add("jscallback", "", m_callback);
    args.add("keep_unspecified", "Write all dimensions", m_writeAllDims, true);
    args.add("order", "Dimension order", m_dimOrder);
    args.add("write_header", "Whether a header should be written",
             m_writeHeader, true);
    args.add("newline", "String to use as newline", m_newline, "\n");
    args.add("delimiter", "Dimension delimiter", m_delimiter, ",");
    args.add("quote_header", "Whether a header should be quoted", m_quoteHeader,
             true);
    args.add("precision", "Output precision", m_precision, 3);
}

TextWriter::DimSpec TextWriter::extractDim(std::string dim, PointTableRef table)
{
    Utils::trim(dim);

    size_t precision(0);
    StringList s = Utils::split(dim, ':');
    if (s.size() == 1)
        precision = m_precision;
    else if (s.size() == 2)
    {
        try
        {
            size_t pos;
            int i = std::stoi(s[1], &pos);
            if (i < 0 || pos != s[1].size())
                throw pdal_error("Dummy"); // Throw to be caught below.
            precision = static_cast<size_t>(i);
        }
        catch (...)
        {
            throwError("Can't convert dimension precision for '" + dim + "'.");
        }
    }
    else
        throwError("Invalid dimension specification '" + dim + "'.");
    Dimension::Id d = table.layout()->findDim(s[0]);
    if (d == Dimension::Id::Unknown)
        throwError("Dimension not found with name '" + dim + "'.");
    return {d, precision, table.layout()->dimName(d)};
}

bool TextWriter::findDim(Dimension::Id id, DimSpec& ds)
{
    auto it = std::find_if(m_dims.begin(), m_dims.end(),
                           [id](const DimSpec& tds) { return tds.id == id; });
    if (it == m_dims.end())
        return false;
    ds = *it;
    return true;
}

void TextWriter::ready(PointTableRef table)
{
    if (m_rustView)
    {
        pdal_point_view_destroy(m_rustView);
        m_rustView = nullptr;
    }
    m_dims.clear();
    m_rustDims.clear();

    // Find the dimensions listed and put them on the id list.
    StringList dimNames = Utils::split2(m_dimOrder, ',');
    for (const std::string& dim : dimNames)
    {
        const DimSpec& spec = extractDim(dim, table);
        m_dims.push_back(spec);
    }

    // Add the rest of the dimensions to the list if we're doing that.
    // Yes, this isn't efficient when, but it's simple.
    if (m_dimOrder.empty() || m_writeAllDims)
    {
        Dimension::IdList all = table.layout()->dims();
        for (auto id : all)
        {
            DimSpec ds{id, static_cast<size_t>(m_precision),
                       table.layout()->dimName(id)};
            if (!findDim(id, ds))
                m_dims.push_back(ds);
        }
    }

    pdal_point_layout_t* rustLayout = pdal_point_layout_create();
    auto registerDim = [&](Dimension::Id id)
    {
        if (std::find(m_rustDims.begin(), m_rustDims.end(), id) !=
            m_rustDims.end())
            return;
        pdal_point_layout_register_dim(rustLayout,
                                       table.layout()->dimName(id).c_str(),
                                       rustTypeId(table.layout()->dimType(id)));
        m_rustDims.push_back(id);
    };

    for (const DimSpec& dim : m_dims)
        registerDim(dim.id);
    if (m_outputType == OutputType::GEOJSON)
    {
        registerDim(Dimension::Id::X);
        registerDim(Dimension::Id::Y);
        registerDim(Dimension::Id::Z);
    }

    m_rustView = pdal_point_view_create(rustLayout);
    m_idx = 0;
}

bool TextWriter::processOne(PointRef& point)
{
    PointId idx = pdal_point_view_add_point(m_rustView);
    for (Dimension::Id dim : m_rustDims)
    {
        pdal_point_view_set_f64(m_rustView, idx, Dimension::name(dim).c_str(),
                                point.getFieldAs<double>(dim));
    }
    m_idx++;
    return true;
}

void TextWriter::write(const PointViewPtr view)
{
    PointRef point(*view, 0);

    for (PointId idx = 0; idx < view->size(); ++idx)
    {
        point.setPointId(idx);
        processOne(point);
    }
}

void TextWriter::done(PointTableRef /*table*/)
{
    pdal_options_t* options = pdal_options_create();
    addOption(options, "filename", filename());
    addOption(
        options, "format",
        std::string(m_outputType == OutputType::GEOJSON ? "geojson" : "csv"));
    addOption(options, "jscallback", m_callback);
    addOption(options, "keep_unspecified", m_writeAllDims);
    addOption(options, "order", m_dimOrder);
    addOption(options, "write_header", m_writeHeader);
    addOption(options, "newline", m_newline);
    addOption(options, "delimiter", m_delimiter);
    addOption(options, "quote_header", m_quoteHeader);
    addOption(options, "precision", m_precision);

    pdal_writer_t* writer = pdal_writer_create_text(options);
    if (!writer)
    {
        pdal_options_destroy(options);
        throwLastRustError("Failed to create Rust text writer.");
    }

    bool ok = pdal_writer_write_view(writer, m_rustView);
    pdal_writer_destroy(writer);
    pdal_options_destroy(options);
    if (!ok)
        throwLastRustError("Rust text writer failed.");

    pdal_point_view_destroy(m_rustView);
    m_rustView = nullptr;
    getMetadata().addList("filename", filename());
}

} // namespace pdal
