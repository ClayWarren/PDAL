/******************************************************************************
 * Copyright (c) 2015, Howard Butler (howard@hobu.co)
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

#include "Ilvis2Reader.hpp"
#include <pdal/util/FileUtils.hpp>
#include <pdal/util/ProgramArgs.hpp>

namespace pdal
{

static StaticPluginInfo const s_info{
    "readers.ilvis2", "ILVIS2 Reader",
    "https://pdal.org/stages/readers.ilvis2.html"};

CREATE_STATIC_STAGE(Ilvis2Reader, s_info)

namespace
{

void addOption(pdal_options_t* options, const std::string& key,
               const std::string& value)
{
    if (!value.empty())
        pdal_options_add_str(options, key.c_str(), value.c_str());
}

void throwLastRustError(const std::string& fallback)
{
    const char* message = pdal_last_error();
    if (message && message[0])
        throw pdal_error(message);
    throw pdal_error(fallback);
}

std::string takeString(char* value)
{
    std::string output(value ? value : "");
    pdal_string_free(value);
    return output;
}

MetadataNode addMetadataChild(MetadataNode& parent,
                              const pdal_metadata_node_t* rustNode)
{
    std::string name = takeString(pdal_metadata_node_name(rustNode));
    std::string description =
        takeString(pdal_metadata_node_description(rustNode));
    uint8_t valueKind = pdal_metadata_node_value_kind(rustNode);

    switch (valueKind)
    {
    case 1:
        return parent.add(name, pdal_metadata_node_value_i64(rustNode),
                          description);
    case 2:
        return parent.add(name, pdal_metadata_node_value_u64(rustNode),
                          description);
    case 3:
        return parent.add(name, pdal_metadata_node_value_f64(rustNode),
                          description);
    case 4:
        return parent.add(name, pdal_metadata_node_value_bool(rustNode),
                          description);
    case 0:
        return parent.add(name, takeString(pdal_metadata_node_value(rustNode)),
                          description);
    default:
        return parent.add(name);
    }
}

void copyMetadataChildren(const pdal_metadata_node_t* rustNode,
                          MetadataNode& cppNode)
{
    uint64_t childCount = pdal_metadata_node_child_count(rustNode);
    for (uint64_t i = 0; i < childCount; ++i)
    {
        pdal_metadata_node_t* rustChild = pdal_metadata_node_child(rustNode, i);
        MetadataNode cppChild = addMetadataChild(cppNode, rustChild);
        copyMetadataChildren(rustChild, cppChild);
        pdal_metadata_node_destroy(rustChild);
    }
}

std::string mappingName(Ilvis2Reader::IlvisMapping mapping)
{
    switch (mapping)
    {
    case Ilvis2Reader::IlvisMapping::LOW:
        return "low";
    case Ilvis2Reader::IlvisMapping::HIGH:
        return "high";
    case Ilvis2Reader::IlvisMapping::ALL:
        return "all";
    case Ilvis2Reader::IlvisMapping::INVALID:
        return "invalid";
    }
    return "invalid";
}

} // namespace

std::string Ilvis2Reader::getName() const
{
    return s_info.name;
}

std::istream& operator>>(std::istream& in, Ilvis2Reader::IlvisMapping& mval)
{
    std::string s;

    in >> s;
    s = Utils::toupper(s);

    static std::map<std::string, Ilvis2Reader::IlvisMapping> m = {
        {"INVALID", Ilvis2Reader::IlvisMapping::INVALID},
        {"LOW", Ilvis2Reader::IlvisMapping::LOW},
        {"HIGH", Ilvis2Reader::IlvisMapping::HIGH},
        {"ALL", Ilvis2Reader::IlvisMapping::ALL}};

    mval = m[s];
    return in;
}

std::ostream& operator<<(std::ostream& out,
                         const Ilvis2Reader::IlvisMapping& mval)
{
    switch (mval)
    {
    case Ilvis2Reader::IlvisMapping::INVALID:
        out << "Invalid";
    case Ilvis2Reader::IlvisMapping::LOW:
        out << "Low";
    case Ilvis2Reader::IlvisMapping::HIGH:
        out << "High";
    case Ilvis2Reader::IlvisMapping::ALL:
        out << "All";
    }
    return out;
}

Ilvis2Reader::Ilvis2Reader() {}

Ilvis2Reader::~Ilvis2Reader()
{
    if (m_rustView)
        pdal_point_view_destroy(m_rustView);
}

void Ilvis2Reader::addArgs(ProgramArgs& args)
{
    args.add("mapping", "Mapping for values", m_mapping, IlvisMapping::ALL);
    args.add("metadata", "Metadata file", m_metadataFile);
}

void Ilvis2Reader::addDimensions(PointLayoutPtr layout)
{
    m_dims = {Dimension::Id::LvisLfid,
              Dimension::Id::ShotNumber,
              Dimension::Id::GpsTime,
              Dimension::Id::LongitudeCentroid,
              Dimension::Id::LatitudeCentroid,
              Dimension::Id::ElevationCentroid,
              Dimension::Id::LongitudeLow,
              Dimension::Id::LatitudeLow,
              Dimension::Id::ElevationLow,
              Dimension::Id::LongitudeHigh,
              Dimension::Id::LatitudeHigh,
              Dimension::Id::ElevationHigh,
              Dimension::Id::X,
              Dimension::Id::Y,
              Dimension::Id::Z};

    for (Dimension::Id dim : m_dims)
        layout->registerDim(dim);
}

void Ilvis2Reader::initialize(PointTableRef)
{
    m_rustIndex = 0;
    if (m_rustView)
    {
        pdal_point_view_destroy(m_rustView);
        m_rustView = nullptr;
    }

    if (!m_metadataFile.empty() && !FileUtils::fileExists(m_metadataFile))
        throwError("Invalid metadata file: '" + m_metadataFile + "'");

    setSpatialReference("EPSG:4326");

    pdal_options_t* options = pdal_options_create();
    addOption(options, "filename", m_filename);
    addOption(options, "mapping", mappingName(m_mapping));
    addOption(options, "metadata", m_metadataFile);

    pdal_reader_t* reader = pdal_reader_create_ilvis2(options);
    if (!reader)
    {
        pdal_options_destroy(options);
        throwLastRustError("Failed to create Rust ILVIS2 reader.");
    }

    m_rustView = pdal_reader_read_first(reader);
    if (!m_rustView)
    {
        pdal_reader_destroy(reader);
        pdal_options_destroy(options);
        throwLastRustError("Rust ILVIS2 reader failed.");
    }

    pdal_metadata_node_t* metadata = pdal_reader_metadata(reader);
    if (metadata)
    {
        copyMetadataChildren(metadata, m_metadata);
        pdal_metadata_node_destroy(metadata);
    }

    pdal_reader_destroy(reader);
    pdal_options_destroy(options);
}

void Ilvis2Reader::ready(PointTableRef)
{
    m_rustIndex = 0;
}

bool Ilvis2Reader::processOne(PointRef& point)
{
    if (m_rustIndex >= pdal_point_view_length(m_rustView))
        return false;

    copyPoint(point);
    m_rustIndex++;
    return true;
}

void Ilvis2Reader::copyPoint(PointRef& point)
{
    for (Dimension::Id dim : m_dims)
    {
        point.setField(dim,
                       pdal_point_view_get_f64(m_rustView, m_rustIndex,
                                               Dimension::name(dim).c_str()));
    }
}

point_count_t Ilvis2Reader::read(PointViewPtr view, point_count_t count)
{
    PointId idx = view->size();
    point_count_t numRead = 0;

    PointRef point(*view, 0);
    while (numRead < count)
    {
        point.setPointId(idx++);
        if (!processOne(point))
            break;
        if (m_cb)
            m_cb(*view, idx);
        numRead++;
    }

    return numRead;
}

void Ilvis2Reader::done(PointTableRef) {}

} // namespace pdal
