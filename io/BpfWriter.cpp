/******************************************************************************
 * Copyright (c) 2015, Hobu Inc., hobu@hobu.co
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

#include "BpfWriter.hpp"

#include <pdal/Options.hpp>
#include <pdal/pdal_features.hpp>
#include <pdal/util/FileUtils.hpp>
#include <pdal/util/ProgramArgs.hpp>

#include <arbiter/arbiter.hpp>
#include <pdal/PointView.hpp>
#include <pdal/util/Utils.hpp>

namespace pdal
{

static StaticPluginInfo const s_info{
    "writers.bpf",
    "\"Binary Point Format\" (BPF) writer support. BPF is a simple \n"
    "DoD and research format that is used by some sensor and \n"
    "processing chains.",
    "https://pdal.org/stages/writers.bpf.html",
    {"bpf"}};

CREATE_STATIC_STAGE(BpfWriter, s_info)

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
    pdal_options_add_str(options, key.c_str(), value ? "true" : "false");
}

void addOption(pdal_options_t* options, const std::string& key, double value)
{
    pdal_options_add_str(options, key.c_str(), Utils::toString(value).c_str());
}

void throwLastRustError(const std::string& fallback)
{
    const char* message = pdal_last_error();
    if (message && message[0])
        throw pdal_error(message);
    throw pdal_error(fallback);
}

std::string formatName(BpfFormat format)
{
    switch (format)
    {
    case BpfFormat::PointMajor:
        return "point";
    case BpfFormat::ByteMajor:
        return "byte";
    case BpfFormat::DimMajor:
        return "dimension";
    }
    return "dimension";
}

std::string joinStrings(const StringList& values)
{
    std::ostringstream oss;
    for (size_t i = 0; i < values.size(); ++i)
    {
        if (i)
            oss << ',';
        oss << values[i];
    }
    return oss.str();
}

} // namespace

std::string BpfWriter::getName() const
{
    return s_info.name;
}

BpfWriter::~BpfWriter()
{
    if (m_rustView)
        pdal_point_view_destroy(m_rustView);
}

std::istream& operator>>(std::istream& in, BpfWriter::CoordId& id)
{
    std::string s;
    in >> s;
    if (s == "auto")
        id.m_auto = true;
    else if (!Utils::fromString(s, id.m_val) || id.m_val < -60 || id.m_val > 60)
        in.setstate(std::ios_base::failbit);
    return in;
}

std::ostream& operator<<(std::ostream& out, const BpfWriter::CoordId& id)
{
    if (id.m_auto)
        out << "auto";
    else
        out << id.m_val;
    return out;
}

void BpfWriter::addArgs(ProgramArgs& args)
{
    args.add("compression", "Output compression", m_compression);
    args.add("header_data", "Base64-encoded header data", m_extraDataSpec);
    args.add("format", "Output format", m_format, BpfFormat::DimMajor);
    args.add("coord_id", "UTM coordinate ID", m_coordId, {true, 0});
    args.add("bundledfile", "List of files to bundle in output",
             m_bundledFilesSpec);
    args.add("output_dims", "Output dimensions", m_outputDims);
    m_scaling.addArgs(args);
}

void BpfWriter::initialize()
{
    // Deal with remote files
    if (Utils::isRemote(filename()))
    {
        // swap our filename for a tmp file
        std::string tmpname = Utils::tempFilename(filename());
        m_remoteFilename = filename();
        setFilename(tmpname);
    }

#ifndef PDAL_HAVE_ZLIB
    if (m_compression)
        throwError("Can't write compressed BPF. PDAL wasn't built with "
                   "Zlib support.");
#endif

    for (const auto& file : m_bundledFilesSpec)
    {
        if (!FileUtils::fileExists(file))
            throwError("Bundledfile '" + file + "' doesn't exist.");

        size_t size = FileUtils::fileSize(file);
        if (size > (std::numeric_limits<uint32_t>::max)())
            throwError("Bundled file '" + file + "' too large.");
        if (size == 0)
            throwError("Bundled file '" + file +
                       "' empty or otherwise invalid.");

        BpfUlemFile ulemFile(size, FileUtils::getFilename(file), file);
        if (ulemFile.m_filename.length() > 32)
            throwError("Bundled file '" + file +
                       "' name exceeds "
                       "maximum length of 32.");
    }

    // BPF coordinates are always in UTM meters, which can be quite large.
    // Allowing the writer to proceed with the default offset of 0 can lead to
    // unexpected quantization of the coordinates. Instead, we force use of
    // auto offset to subtract the minimum value in XYZ, unless of course, the
    // user chooses to override with their own offset.
    if (!m_scaling.m_xOffArg->set())
        m_scaling.m_xXform.m_offset.m_auto = true;
    if (!m_scaling.m_yOffArg->set())
        m_scaling.m_yXform.m_offset.m_auto = true;
    if (!m_scaling.m_zOffArg->set())
        m_scaling.m_zXform.m_offset.m_auto = true;
}

void BpfWriter::prepared(PointTableRef table)
{
    loadBpfDimensions(table.layout());
}

void BpfWriter::readyFile(const std::string& filename,
                          const SpatialReference& srs)
{
    m_curFilename = filename;
    m_curSrs = srs;
    if (m_rustView)
    {
        pdal_point_view_destroy(m_rustView);
        m_rustView = nullptr;
    }

    pdal_point_layout_t* rustLayout = pdal_point_layout_create();
    for (size_t i = 0; i < m_dims.size(); ++i)
        pdal_point_layout_register_dim(rustLayout, m_dims[i].m_label.c_str(),
                                       rustTypeId(m_dimTypes[i]));
    m_rustView = pdal_point_view_create(rustLayout);
}

void BpfWriter::loadBpfDimensions(PointLayoutPtr layout)
{
    Dimension::IdList dims;
    m_dims.clear();
    m_dimTypes.clear();

    if (m_outputDims.size())
    {
        for (std::string& s : m_outputDims)
        {
            Dimension::Id id = layout->findDim(s);
            if (id == Dimension::Id::Unknown)
                throwError("Invalid dimension '" + s +
                           "' specified for "
                           "'output_dims' option.");
            dims.push_back(id);
        }
    }
    else
        dims = layout->dims();

    // Verify that we have X, Y and Z and that they're the first three
    // dimensions.
    std::sort(dims.begin(), dims.end());
    if (dims.size() < 3 || dims[0] != Dimension::Id::X ||
        dims[1] != Dimension::Id::Y || dims[2] != Dimension::Id::Z)
    {
        throwError("Missing one of dimensions X, Y or Z.  Can't write BPF.");
    }

    for (auto id : dims)
    {
        BpfDimension dim;
        dim.m_id = id;
        dim.m_label = layout->dimName(id);
        m_dims.push_back(dim);
        m_dimTypes.push_back(layout->dimType(id));
    }
}

void BpfWriter::prerunFile(const PointViewSet& pvSet)
{
    m_scaling.setAutoXForm(pvSet);
}

void BpfWriter::writeView(const PointViewPtr dataShared)
{
    copyViewToRust(dataShared);
}

void BpfWriter::copyViewToRust(const PointViewPtr data)
{
    for (PointId idx = 0; idx < data->size(); ++idx)
    {
        PointId outIdx = pdal_point_view_add_point(m_rustView);
        for (BpfDimension& dim : m_dims)
        {
            pdal_point_view_set_f64(m_rustView, outIdx, dim.m_label.c_str(),
                                    data->getFieldAs<double>(dim.m_id, idx));
        }
    }
}

void BpfWriter::writeRustView()
{
    pdal_options_t* options = pdal_options_create();
    addOption(options, "filename", m_curFilename);
    addOption(options, "compression", m_compression);
    addOption(options, "format", formatName(m_format));
    addOption(options, "header_data", m_extraDataSpec);
    addOption(options, "output_dims", joinStrings(m_outputDims));
    addOption(options, "scale_x", m_scaling.m_xXform.m_scale.m_val);
    addOption(options, "scale_y", m_scaling.m_yXform.m_scale.m_val);
    addOption(options, "scale_z", m_scaling.m_zXform.m_scale.m_val);
    addOption(options, "offset_x", m_scaling.m_xXform.m_offset.m_val);
    addOption(options, "offset_y", m_scaling.m_yXform.m_offset.m_val);
    addOption(options, "offset_z", m_scaling.m_zXform.m_offset.m_val);

    int coordId = m_coordId.m_val;
    if (m_coordId.m_auto && !coordId)
    {
        int32_t zone = 0;
        if (pdal_srs_get_utm_zone(m_curSrs.getWKT().c_str(), &zone) && zone)
            coordId = zone;
    }
    addOption(options, "coord_id", Utils::toString(coordId));

    for (const std::string& bundledFile : m_bundledFilesSpec)
        addOption(options, "bundledfile", bundledFile);

    pdal_writer_t* writer = pdal_writer_create_bpf(options);
    if (!writer)
    {
        pdal_options_destroy(options);
        throwLastRustError("Failed to create Rust BPF writer.");
    }

    bool ok = pdal_writer_write_view(writer, m_rustView);
    pdal_writer_destroy(writer);
    pdal_options_destroy(options);
    if (!ok)
        throwLastRustError("Rust BPF writer failed.");
}

void BpfWriter::doneFile()
{
    writeRustView();
    pdal_point_view_destroy(m_rustView);
    m_rustView = nullptr;
    getMetadata().addList("filename", m_curFilename);

    if (m_remoteFilename.size())
    {
        arbiter::Arbiter a;
        a.put(m_remoteFilename, a.getBinary(filename()));

        // Clean up temporary
        FileUtils::deleteFile(filename());

        // Set the remote filename back as the filename and clear.
        setFilename(m_remoteFilename);
        m_remoteFilename.clear();
    }
}

} // namespace pdal
