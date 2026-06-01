/******************************************************************************
 * Copyright (c) 2025, Isaac Bell (isaac@hobu.co)
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
 *       the documentation and/or other materials provided with the
 *       distribution.
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

#include "SpzWriter.hpp"

#include <pdal/private/RustViewConverter.hpp>

namespace pdal
{

static StaticPluginInfo const s_info{"writers.spz",
                                     "SPZ writer",
                                     "https://pdal.org/stages/writers.spz.html",
                                     {"spz"}};

CREATE_SHARED_STAGE(SpzWriter, s_info)

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

} // namespace

std::string SpzWriter::getName() const
{
    return s_info.name;
}

void SpzWriter::addArgs(ProgramArgs& args)
{
    args.add("antialiased", "Mark the data as antialiased", m_antialiased);
}

void SpzWriter::prepared(PointTableRef table)
{
    m_coordinateOrientation =
        table.metadata().findChild("coordinate_orientation").value();
}

void SpzWriter::write(const PointViewPtr data)
{
    pdal_point_view_t* rustView = rust_view_converter::toRust(data);

    pdal_options_t* options = pdal_options_create();
    addOption(options, "filename", filename());
    addOption(options, "antialiased", m_antialiased);
    if (!m_coordinateOrientation.empty())
        addOption(options, "coordinate_orientation", m_coordinateOrientation);

    pdal_writer_t* writer = pdal_writer_create_spz(options);
    if (!writer)
    {
        pdal_point_view_destroy(rustView);
        pdal_options_destroy(options);
        rust_view_converter::throwLastError(
            "Failed to create Rust SPZ writer.");
    }

    bool ok = pdal_writer_write_view(writer, rustView);
    pdal_writer_destroy(writer);
    pdal_point_view_destroy(rustView);
    pdal_options_destroy(options);
    if (!ok)
        rust_view_converter::throwLastError("Rust SPZ writer failed.");
}

void SpzWriter::done(PointTableRef)
{
    getMetadata().addList("filename", filename());
}

} // namespace pdal
