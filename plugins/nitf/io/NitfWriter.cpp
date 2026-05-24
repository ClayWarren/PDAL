/****************************************************************************
 * Copyright (c) 2012, Michael P. Gerlek (mpg@flaxen.com)
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
 *     * Neither the name of Hobu, Inc. or Flaxen Consulting LLC nor the
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

#include <atomic>
#include <cstdio>
#include <filesystem>
#include <memory>
#include <vector>

#include "NitfWriter.hpp"

#include <io/private/las/Header.hpp>
#include <pdal/PointView.hpp>
#include <pdal/private/gdal/GDALUtils.hpp>
#include <pdal/util/FileUtils.hpp>

#include <pdal_capi.h>

namespace pdal
{

static PluginInfo const s_info{"writers.nitf", "NITF Writer",
                               "https://pdal.org/stages/writers.nitf.html"};

CREATE_SHARED_STAGE(NitfWriter, s_info)

std::string NitfWriter::getName() const
{
    return s_info.name;
}

BOX3D NitfWriter::reprojectBoxToDD(const SpatialReference& reference,
                                   const BOX3D& box)
{
    if (reference.empty())
        return BOX3D();

    BOX3D output(box);
    if (!gdal::reprojectBounds(output, reference.getWKT(), "EPSG:4326"))
        throwError("Couldn't reproject corner points to geographic: " +
                   gdal::lastError());
    return output;
}

NitfWriter::NitfWriter()
{
    try
    {
        m_nitf.initialize();
    }
    catch (const NitfFileWriter::error& err)
    {
        throwError(err.what());
    }
}

void NitfWriter::addArgs(ProgramArgs& args)
{
    LasWriter::addArgs(args);
    m_nitf.addArgs(args);
}

void NitfWriter::writeView(const PointViewPtr view)
{
    LasWriter::writeView(view);
}

namespace
{

std::string make_temp_las_path(const std::string& hint)
{
    auto dir = std::filesystem::temp_directory_path();
    static std::atomic<uint64_t> counter{0};
    uint64_t id = counter.fetch_add(1) ^
                  static_cast<uint64_t>(reinterpret_cast<uintptr_t>(&counter));
    std::string base = "pdal_nitf_payload_" + std::to_string(id) + "_" + hint
                       + ".las";
    return (dir / base).string();
}

} // namespace

void NitfWriter::readyFile(const std::string& filename,
                           const SpatialReference& srs)
{
    // Final NITF output filename comes from `filename`. The embedded LAS
    // payload is staged in a temp file that LasWriter writes through its
    // standard (now Rust-backed) flow.
    m_nitf.setFilename(filename);
    m_payloadPath = make_temp_las_path(
        std::filesystem::path(filename).stem().string());
    LasWriter::readyFile(m_payloadPath, srs);
}

void NitfWriter::doneFile()
{
    // Have LasWriter finalize the LAS payload to the temp path.
    LasWriter::doneFile();

    BOX3D bounds = reprojectBoxToDD(m_srs, header().bounds);

    pdal_nitf_write_options_t opts{};
    std::string title =
        m_nitf.m_fileTitle.empty() ? FileUtils::getFilename(m_nitf.m_filename)
                                   : m_nitf.m_fileTitle;
    opts.file_title = title.c_str();
    opts.complexity_level =
        m_nitf.m_cLevel.empty() ? nullptr : m_nitf.m_cLevel.c_str();
    opts.system_type =
        m_nitf.m_sType.empty() ? nullptr : m_nitf.m_sType.c_str();
    opts.origin_station_id =
        m_nitf.m_oStationId.empty() ? nullptr : m_nitf.m_oStationId.c_str();
    opts.file_class =
        m_nitf.m_fileClass.empty() ? nullptr : m_nitf.m_fileClass.c_str();
    opts.origin_name =
        m_nitf.m_origName.empty() ? nullptr : m_nitf.m_origName.c_str();
    opts.origin_phone =
        m_nitf.m_origPhone.empty() ? nullptr : m_nitf.m_origPhone.c_str();
    opts.fsclsy = m_nitf.m_securityClassificationSystem.empty()
                      ? nullptr
                      : m_nitf.m_securityClassificationSystem.c_str();
    opts.fsctlh = m_nitf.m_securityControlAndHandling.empty()
                      ? nullptr
                      : m_nitf.m_securityControlAndHandling.c_str();
    opts.fscltx = m_nitf.m_sic.empty() ? nullptr : m_nitf.m_sic.c_str();
    opts.image_security_class =
        m_nitf.m_imgSecurityClass.empty() ? nullptr
                                          : m_nitf.m_imgSecurityClass.c_str();
    opts.image_date_time =
        m_nitf.m_imgDate.empty() ? nullptr : m_nitf.m_imgDate.c_str();
    opts.image_id2 = m_nitf.m_imgIdentifier2.empty()
                         ? nullptr
                         : m_nitf.m_imgIdentifier2.c_str();

    std::vector<const char*> aimidb_ptrs;
    aimidb_ptrs.reserve(m_nitf.m_aimidb.size() + 1);
    for (const auto& s : m_nitf.m_aimidb)
        aimidb_ptrs.push_back(s.c_str());
    aimidb_ptrs.push_back(nullptr);

    std::vector<const char*> acftb_ptrs;
    acftb_ptrs.reserve(m_nitf.m_acftb.size() + 1);
    for (const auto& s : m_nitf.m_acftb)
        acftb_ptrs.push_back(s.c_str());
    acftb_ptrs.push_back(nullptr);

    opts.aimidb = m_nitf.m_aimidb.empty() ? nullptr : aimidb_ptrs.data();
    opts.acftb = m_nitf.m_acftb.empty() ? nullptr : acftb_ptrs.data();
    opts.minx = bounds.minx;
    opts.miny = bounds.miny;
    opts.maxx = bounds.maxx;
    opts.maxy = bounds.maxy;

    bool ok = pdal_nitf_write(m_payloadPath.c_str(), m_nitf.m_filename.c_str(),
                              &opts);
    std::remove(m_payloadPath.c_str());
    m_payloadPath.clear();
    if (!ok)
        throwError(pdal_last_error());
}

} // namespace pdal
