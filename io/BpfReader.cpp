/******************************************************************************
 * Copyright (c) 2014, Andrew Bell
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

#include "BpfReader.hpp"

#include <arbiter/arbiter.hpp>
#include <pdal/Options.hpp>
#include <pdal/util/FileUtils.hpp>

namespace pdal
{

static StaticPluginInfo const s_info{
    "readers.bpf",
    "\"Binary Point Format\" (BPF) reader support. BPF is a simple \n"
    "DoD and research format that is used by some sensor and \n"
    "processing chains.",
    "https://pdal.org/stages/readers.bpf.html",
    {"bpf"}};

CREATE_STATIC_STAGE(BpfReader, s_info)

struct BpfReader::Args
{
    bool m_fixNames;
};

std::string BpfReader::getName() const
{
    return s_info.name;
}

BpfReader::BpfReader() : m_args(new BpfReader::Args) {}

BpfReader::~BpfReader()
{
    if (m_rustView)
        pdal_point_view_destroy(m_rustView);
    cleanupRemoteFile();
}

namespace
{

void addOption(pdal_options_t* options, const std::string& key,
               const std::string& value)
{
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
                          description, 17);
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

} // namespace

void BpfReader::addArgs(ProgramArgs& args)
{
    args.add("fix_dims",
             "Make invalid dimension names valid by changing "
             "invalid characters to '_'",
             m_args->m_fixNames, true);
}

void BpfReader::addDimensions(PointLayoutPtr layout)
{
    m_rustDims.clear();
    m_rustDimNames.clear();
    uint64_t dimCount = pdal_point_view_dim_count(m_rustView);
    for (uint64_t idx = 0; idx < dimCount; ++idx)
    {
        char* rawName = pdal_point_view_dim_name(m_rustView, idx);
        if (!rawName)
            continue;
        std::string name(rawName);
        pdal_string_free(rawName);
        Dimension::Id id =
            layout->registerOrAssignDim(name, Dimension::Type::Float);
        m_rustDims.push_back(id);
        m_rustDimNames.push_back(name);
    }
}

QuickInfo BpfReader::inspect()
{
    QuickInfo qi;

    initialize();
    qi.m_valid = true;
    qi.m_pointCount = pdal_point_view_length(m_rustView);
    pdal_spatial_reference_t* srs =
        pdal_point_view_spatial_reference(m_rustView);
    if (srs)
    {
        char* text = pdal_spatial_reference_text(srs);
        qi.m_srs.set(text ? text : "");
        pdal_string_free(text);
        pdal_spatial_reference_destroy(srs);
    }

    for (MetadataNode& dim : m_metadata.children("dimension"))
    {
        MetadataNode name = dim.findChild("name");
        if (name)
            qi.m_dimNames.push_back(name.value());

        MetadataNode min = dim.findChild("min");
        MetadataNode max = dim.findChild("max");
        if (name && min && max)
        {
            if (name.value() == "X")
            {
                qi.m_bounds.minx = min.value<double>();
                qi.m_bounds.maxx = max.value<double>();
            }
            if (name.value() == "Y")
            {
                qi.m_bounds.miny = min.value<double>();
                qi.m_bounds.maxy = max.value<double>();
            }
            if (name.value() == "Z")
            {
                qi.m_bounds.minz = min.value<double>();
                qi.m_bounds.maxz = max.value<double>();
            }
        }
    }
    return qi;
}

// When the stage is intialized, the schema needs to be populated with the
// dimensions in order to allow subsequent stages to be aware of or append to
// the dimensions in the PointView.
void BpfReader::initialize()
{
    if (m_filename.empty())
        throwError("Can't read BPF file without filename.");

    if (m_remoteFilename.empty() && Utils::isRemote(m_filename))
    {
        std::string tmpname = Utils::tempFilename(m_filename);
        m_remoteFilename = m_filename;
        m_filename = tmpname;
        arbiter::Arbiter arbiter;
        arbiter.put(m_filename, arbiter.getBinary(m_remoteFilename));
    }

    if (m_rustView)
    {
        pdal_point_view_destroy(m_rustView);
        m_rustView = nullptr;
    }
    m_rustIndex = 0;

    pdal_options_t* options = pdal_options_create();
    addOption(options, "filename", m_filename);
    addOption(options, "fix_dims", m_args->m_fixNames ? "true" : "false");

    pdal_reader_t* reader = pdal_reader_create_bpf(options);
    if (!reader)
    {
        pdal_options_destroy(options);
        throwLastRustError("Failed to create Rust BPF reader.");
    }

    m_rustView = pdal_reader_read_first(reader);
    if (!m_rustView)
    {
        pdal_reader_destroy(reader);
        pdal_options_destroy(options);
        throwLastRustError("Rust BPF reader failed.");
    }

    pdal_spatial_reference_t* srs =
        pdal_point_view_spatial_reference(m_rustView);
    if (srs)
    {
        char* text = pdal_spatial_reference_text(srs);
        SpatialReference spatialRef(text ? text : "");
        setSpatialReference(spatialRef);
        pdal_string_free(text);
        pdal_spatial_reference_destroy(srs);
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

void BpfReader::ready(PointTableRef)
{
    m_rustIndex = 0;
}

void BpfReader::done(PointTableRef)
{
    cleanupRemoteFile();
}

bool BpfReader::processOne(PointRef& point)
{
    if (m_rustIndex >= pdal_point_view_length(m_rustView) ||
        m_rustIndex >= m_count)
        return false;

    copyRustPoint(point, m_rustIndex);
    ++m_rustIndex;
    return true;
}

point_count_t BpfReader::read(PointViewPtr data, point_count_t count)
{
    point_count_t cnt = 0;
    while (cnt < count && m_rustIndex < pdal_point_view_length(m_rustView))
    {
        PointRef point(*data, data->size());
        copyRustPoint(point, m_rustIndex);
        ++m_rustIndex;
        ++cnt;
    }
    return cnt;
}

void BpfReader::copyRustPoint(PointRef& point, PointId rustIndex)
{
    for (size_t dimIdx = 0; dimIdx < m_rustDims.size(); ++dimIdx)
    {
        point.setField(m_rustDims[dimIdx],
                       pdal_point_view_get_f64(m_rustView, rustIndex,
                                               m_rustDimNames[dimIdx].c_str()));
    }
}

bool BpfReader::eof()
{
    return m_rustIndex >= numPoints();
}

void BpfReader::cleanupRemoteFile()
{
    if (!Utils::isRemote(m_remoteFilename))
        return;

    FileUtils::deleteFile(m_filename);
    m_filename = m_remoteFilename;
    m_remoteFilename.clear();
}

} // namespace pdal
