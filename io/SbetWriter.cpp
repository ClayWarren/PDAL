/******************************************************************************
 * Copyright (c) 2014, Peter J. Gadomski (pete.gadomski@gmail.com)
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

#include "SbetWriter.hpp"

#include <pdal/PointLayout.hpp>
#include <pdal/PointView.hpp>
#include <pdal/util/ProgramArgs.hpp>

namespace pdal
{

static StaticPluginInfo const s_info{
    "writers.sbet",
    "SBET Writer",
    "https://pdal.org/stages/writers.sbet.html",
    {"sbet"}};

CREATE_STATIC_STAGE(SbetWriter, s_info)

namespace
{

void addOption(pdal_options_t* options, const std::string& key,
               const std::string& value)
{
    pdal_options_add_str(options, key.c_str(), value.c_str());
}

void addOption(pdal_options_t* options, const std::string& key, bool value)
{
    addOption(options, key, std::string(value ? "true" : "false"));
}

void throwLastRustError(const std::string& fallback)
{
    const char* message = pdal_last_error();
    if (message && message[0])
        throw pdal_error(message);
    throw pdal_error(fallback);
}

} // namespace

SbetWriter::~SbetWriter()
{
    if (m_rustView)
        pdal_point_view_destroy(m_rustView);
}

std::string SbetWriter::getName() const
{
    return s_info.name;
}

void SbetWriter::addArgs(ProgramArgs& args)
{
    args.add("angles_are_degrees",
             "Angles coming into the writer are in degrees", m_anglesAreDegrees,
             true);
}

void SbetWriter::ready(PointTableRef)
{
    if (m_rustView)
    {
        pdal_point_view_destroy(m_rustView);
        m_rustView = nullptr;
    }

    m_dims = sbet::fileDimensions();
    pdal_point_layout_t* layout = pdal_point_layout_create();
    for (Dimension::Id dim : m_dims)
        pdal_point_layout_register_dim(layout, Dimension::name(dim).c_str(), 9);
    m_rustView = pdal_point_view_create(layout);
}

void SbetWriter::write(const PointViewPtr view)
{
    for (PointId idx = 0; idx < view->size(); ++idx)
    {
        PointId outIdx = pdal_point_view_add_point(m_rustView);
        for (Dimension::Id dim : m_dims)
        {
            double value =
                (view->hasDim(dim) ? view->getFieldAs<double>(dim, idx) : 0.0);
            pdal_point_view_set_f64(m_rustView, outIdx,
                                    Dimension::name(dim).c_str(), value);
        }
    }
}

void SbetWriter::done(PointTableRef table)
{
    pdal_options_t* options = pdal_options_create();
    addOption(options, "filename", filename());
    addOption(options, "angles_are_degrees", m_anglesAreDegrees);

    pdal_writer_t* writer = pdal_writer_create_sbet(options);
    if (!writer)
    {
        pdal_options_destroy(options);
        throwLastRustError("Failed to create Rust SBET writer.");
    }

    bool ok = pdal_writer_write_view(writer, m_rustView);
    pdal_writer_destroy(writer);
    pdal_options_destroy(options);
    if (!ok)
        throwLastRustError("Rust SBET writer failed.");

    pdal_point_view_destroy(m_rustView);
    m_rustView = nullptr;
    getMetadata().addList("filename", filename());
}

} // namespace pdal
