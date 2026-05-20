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

#include <climits>

#include <arbiter/arbiter.hpp>
#include <pdal/Options.hpp>
#include <pdal/pdal_features.hpp>
#include <pdal/util/FileUtils.hpp>

#ifdef PDAL_HAVE_ZLIB
#include <zlib.h>
#endif

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
#ifdef PDAL_HAVE_ZLIB
    if (m_header.m_compression)
    {
        for (auto& stream : m_streams)
        {
            delete stream->popStream();
        }
    }
#endif
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

    std::istream* streamPtr = Utils::openFile(m_filename);
    if (!streamPtr)
        throwError("Can't open file '" + m_filename + "'.");
    ILeStream stream(streamPtr);
    BpfHeader header;
    header.setLog(log());
    BpfDimensionList dims;
    try
    {
        if (!header.read(stream) ||
            !header.readDimensions(stream, dims, m_args->m_fixNames))
            throwError("Couldn't read BPF header.");
    }
    catch (const BpfHeader::error& err)
    {
        Utils::closeFile(streamPtr);
        throwError(err.what());
    }
    Utils::closeFile(streamPtr);

    for (BpfDimension& dim : dims)
    {
        qi.m_dimNames.push_back(dim.m_label);
        if (dim.m_label == "X")
        {
            qi.m_bounds.minx = dim.m_min;
            qi.m_bounds.maxx = dim.m_max;
        }
        if (dim.m_label == "Y")
        {
            qi.m_bounds.miny = dim.m_min;
            qi.m_bounds.maxy = dim.m_max;
        }
        if (dim.m_label == "Z")
        {
            qi.m_bounds.minz = dim.m_min;
            qi.m_bounds.maxz = dim.m_max;
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

bool BpfReader::readUlemData()
{
    if (!m_ulemHeader.read(m_stream))
        return false;

    for (size_t i = 0; i < m_ulemHeader.m_numFrames; i++)
    {
        BpfUlemFrame frame;
        if (!frame.read(m_stream))
            return false;
        m_ulemFrames.push_back(frame);
    }
    return (bool)m_stream;
}

bool BpfReader::readUlemFiles()
{
    BpfUlemFile file;
    while (file.read(m_stream))
    {
        MetadataNode m = m_metadata.add("bundled_file");
        m.addEncoded(file.m_filename, (const unsigned char*)file.m_buf.data(),
                     file.m_len);
    }
    return (bool)m_stream;
}

/// Encode all data that follows the headers as metadata->
/// \return  Whether the stream is still valid.
bool BpfReader::readHeaderExtraData()
{
    if (m_stream.position() < m_header.m_len)
    {
        std::streampos size = m_header.m_len - m_stream.position();
        std::vector<uint8_t> buf(size);
        m_stream.get(buf);
        m_metadata.addEncoded("header_data", buf.data(), buf.size());
    }
    return (bool)m_stream;
}

bool BpfReader::readPolarData()
{
    if (!m_polarHeader.read(m_stream))
        return false;
    for (size_t i = 0; i < m_polarHeader.m_numFrames; ++i)
    {
        BpfPolarFrame frame;
        if (!frame.read(m_stream))
            return false;
        m_polarFrames.push_back(frame);
    }
    return (bool)m_stream;
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

void BpfReader::readPointMajor(PointRef& point)
{
    double x(0), y(0), z(0);

    seekPointMajor(m_index);
    for (size_t dim = 0; dim < m_dims.size(); ++dim)
    {
        float f;

        m_stream >> f;
        double d = f + m_dims[dim].m_offset;
        if (m_dims[dim].m_id == Dimension::Id::X)
            x = d;
        else if (m_dims[dim].m_id == Dimension::Id::Y)
            y = d;
        else if (m_dims[dim].m_id == Dimension::Id::Z)
            z = d;
        else
            point.setField(m_dims[dim].m_id, d);
    }

    m_header.m_xform.apply(x, y, z);
    point.setField(Dimension::Id::X, x);
    point.setField(Dimension::Id::Y, y);
    point.setField(Dimension::Id::Z, z);
    m_index++;
}

point_count_t BpfReader::readPointMajor(PointViewPtr view, point_count_t count)
{
    PointId nextId = view->size();
    PointId idx = m_index;
    point_count_t numRead = 0;
    seekPointMajor(idx);
    while (numRead < count && idx < numPoints())
    {
        for (size_t d = 0; d < m_dims.size(); ++d)
        {
            float f;

            m_stream >> f;
            view->setField(m_dims[d].m_id, nextId, f + m_dims[d].m_offset);
        }

        // Transformation only applies to X, Y and Z
        double x = view->getFieldAs<double>(Dimension::Id::X, nextId);
        double y = view->getFieldAs<double>(Dimension::Id::Y, nextId);
        double z = view->getFieldAs<double>(Dimension::Id::Z, nextId);
        m_header.m_xform.apply(x, y, z);
        view->setField(Dimension::Id::X, nextId, x);
        view->setField(Dimension::Id::Y, nextId, y);
        view->setField(Dimension::Id::Z, nextId, z);
        if (m_cb)
            m_cb(*view, nextId);

        idx++;
        numRead++;
        nextId++;
    }
    m_index = idx;
    return numRead;
}

void BpfReader::readDimMajor(PointRef& point)
{
    if (m_streams.empty())
    {
        for (std::size_t dim(0); dim < m_dims.size(); ++dim)
        {
            std::streamoff offset = sizeof(float) * dim * numPoints();

            m_streams.emplace_back(new ILeStream());
            m_streams.back()->open(m_filename);

#ifdef PDAL_HAVE_ZLIB
            if (m_header.m_compression)
            {
                m_charbufs.emplace_back(new Charbuf());
                m_charbufs.back()->initialize(m_deflateBuf.data(),
                                              m_deflateBuf.size(), m_start);

                m_streams.back()->pushStream(
                    new std::istream(m_charbufs.back().get()));
            }
#endif // PDAL_HAVE_ZLIB

            m_streams.back()->seek(m_start + offset);
        }
    }

    double x(0), y(0), z(0);
    float f(0);
    double d(0);

    for (size_t dim = 0; dim < m_dims.size(); ++dim)
    {
        *m_streams[dim] >> f;
        d = f + m_dims[dim].m_offset;
        if (m_dims[dim].m_id == Dimension::Id::X)
            x = d;
        else if (m_dims[dim].m_id == Dimension::Id::Y)
            y = d;
        else if (m_dims[dim].m_id == Dimension::Id::Z)
            z = d;
        else
            point.setField(m_dims[dim].m_id, d);
    }

    // Transformation only applies to X, Y and Z
    m_header.m_xform.apply(x, y, z);
    point.setField(Dimension::Id::X, x);
    point.setField(Dimension::Id::Y, y);
    point.setField(Dimension::Id::Z, z);
    m_index++;
}

point_count_t BpfReader::readDimMajor(PointViewPtr data, point_count_t count)
{
    PointId idx(0);
    PointId startId = data->size();
    point_count_t numRead = 0;
    for (size_t d = 0; d < m_dims.size(); ++d)
    {
        idx = m_index;
        PointId nextId = startId;
        numRead = 0;
        seekDimMajor(d, idx);
        for (; numRead < count && idx < numPoints(); idx++, numRead++, nextId++)
        {
            float f;

            m_stream >> f;
            data->setField(m_dims[d].m_id, nextId, f + m_dims[d].m_offset);
        }
    }
    m_index = idx;

    // Transformation only applies to X, Y and Z
    for (PointId idx = startId; idx < data->size(); idx++)
    {
        double x = data->getFieldAs<double>(Dimension::Id::X, idx);
        double y = data->getFieldAs<double>(Dimension::Id::Y, idx);
        double z = data->getFieldAs<double>(Dimension::Id::Z, idx);
        m_header.m_xform.apply(x, y, z);
        data->setField(Dimension::Id::X, idx, x);
        data->setField(Dimension::Id::Y, idx, y);
        data->setField(Dimension::Id::Z, idx, z);

        if (m_cb)
            m_cb(*data, idx);
    }

    return numRead;
}

void BpfReader::readByteMajor(PointRef& point)
{
    // We need a temp buffer for the point data
    union uu
    {
        float f;
        uint32_t u32;
    } u;
    double x(0), y(0), z(0);
    uint8_t u8;

    for (size_t dim = 0; dim < m_dims.size(); ++dim)
    {
        u.u32 = 0;
        for (size_t b = 0; b < sizeof(float); ++b)
        {
            seekByteMajor(dim, b, m_index);

            m_stream >> u8;
            u.u32 |= ((uint32_t)u8 << (b * CHAR_BIT));
        }
        double d = u.f + m_dims[dim].m_offset;
        if (m_dims[dim].m_id == Dimension::Id::X)
            x = d;
        else if (m_dims[dim].m_id == Dimension::Id::Y)
            y = d;
        else if (m_dims[dim].m_id == Dimension::Id::Z)
            z = d;
        else
            point.setField(m_dims[dim].m_id, d);
    }

    m_header.m_xform.apply(x, y, z);
    point.setField(Dimension::Id::X, x);
    point.setField(Dimension::Id::Y, y);
    point.setField(Dimension::Id::Z, z);
    m_index++;
}

point_count_t BpfReader::readByteMajor(PointViewPtr data, point_count_t count)
{
    PointId idx(0);
    PointId startId = data->size();
    point_count_t numRead = 0;

    // We need a temp buffer for the point data
    union uu
    {
        float f;
        uint32_t u32;
    };
    std::unique_ptr<union uu[]> uArr(
        new uu[(std::min)(count, numPoints() - m_index)]);

    for (size_t d = 0; d < m_dims.size(); ++d)
    {
        for (size_t b = 0; b < sizeof(float); ++b)
        {
            idx = m_index;
            numRead = 0;
            PointId nextId = startId;
            seekByteMajor(d, b, idx);

            for (; numRead < count && idx < numPoints();
                 idx++, numRead++, nextId++)
            {
                union uu& u = *(uArr.get() + numRead);

                if (b == 0)
                    u.u32 = 0;
                uint8_t u8;
                m_stream >> u8;
                u.u32 |= ((uint32_t)u8 << (b * CHAR_BIT));
                if (b == 3)
                {
                    u.f += static_cast<float>(m_dims[d].m_offset);
                    data->setField(m_dims[d].m_id, nextId, u.f);
                }
            }
        }
    }
    m_index = idx;

    // Transformation only applies to X, Y and Z
    for (PointId idx = startId; idx < data->size(); idx++)
    {
        double x = data->getFieldAs<double>(Dimension::Id::X, idx);
        double y = data->getFieldAs<double>(Dimension::Id::Y, idx);
        double z = data->getFieldAs<double>(Dimension::Id::Z, idx);
        m_header.m_xform.apply(x, y, z);
        data->setField(Dimension::Id::X, idx, x);
        data->setField(Dimension::Id::Y, idx, y);
        data->setField(Dimension::Id::Z, idx, z);

        if (m_cb)
            m_cb(*data, idx);
    }

    return numRead;
}

void BpfReader::seekPointMajor(PointId ptIdx)
{
    std::streamoff offset = ptIdx * sizeof(float) * m_dims.size();
    m_stream.seek(m_start + offset);
}

void BpfReader::seekDimMajor(size_t dimIdx, PointId ptIdx)
{
    std::streamoff offset =
        ((sizeof(float) * dimIdx * numPoints()) + (sizeof(float) * ptIdx));
    m_stream.seek(m_start + offset);
}

void BpfReader::seekByteMajor(size_t dimIdx, size_t byteIdx, PointId ptIdx)
{
    std::streamoff offset = (dimIdx * numPoints() * sizeof(float)) +
                            (byteIdx * numPoints()) + ptIdx;
    m_stream.seek(m_start + offset);
}

#ifdef PDAL_HAVE_ZLIB
size_t BpfReader::readBlock(std::vector<char>& outBuf, size_t index)
{
    uint32_t finalBytes;
    uint32_t compressBytes;

    m_stream >> finalBytes;
    m_stream >> compressBytes;

    std::vector<char> in(compressBytes);

    // Fill the input bytes from the stream.
    m_stream.get(in);
    int ret =
        inflate(in.data(), compressBytes, outBuf.data() + index, finalBytes);
    return (ret ? 0 : finalBytes);
}

int BpfReader::inflate(char* buf, uint32_t insize, char* outbuf,
                       uint32_t outsize)
{
    if (insize == 0)
        return 0;

    int ret;
    z_stream strm;

    /* allocate inflate state */
    strm.zalloc = Z_NULL;
    strm.zfree = Z_NULL;
    strm.opaque = Z_NULL;
    strm.avail_in = 0;
    strm.next_in = Z_NULL;
    if (inflateInit(&strm) != Z_OK)
        return -2;

    strm.avail_in = insize;
    strm.next_in = (unsigned char*)buf;
    strm.avail_out = outsize;
    strm.next_out = (unsigned char*)outbuf;

    ret = ::inflate(&strm, Z_NO_FLUSH);
    (void)inflateEnd(&strm);
    return ret == Z_STREAM_END ? 0 : -1;
}
#endif // PDAL_HAVE_ZLIB

} // namespace pdal
