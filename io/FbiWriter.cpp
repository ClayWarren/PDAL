/******************************************************************************
 * Copyright (c) 2021, Antoine Lavenant, antoine.lavenant@ign.fr
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

#include "FbiWriter.hpp"

#include <pdal/PointView.hpp>

namespace pdal
{

namespace
{
const StaticPluginInfo s_info{"writers.fbi", "FBI Writer",
                              "https://pdal.org/stages/writers.fbi.html"};

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

void throwLastRustError(const std::string& fallback)
{
    const char* message = pdal_last_error();
    if (message && message[0])
        throw pdal_error(message);
    throw pdal_error(fallback);
}

} // namespace

CREATE_STATIC_STAGE(FbiWriter, s_info)

FbiWriter::FbiWriter() {}

FbiWriter::~FbiWriter()
{
    if (m_rustView)
        pdal_point_view_destroy(m_rustView);
}

std::string FbiWriter::getName() const
{
    return s_info.name;
}

void FbiWriter::ready(PointTableRef table)
{
    if (m_rustView)
    {
        pdal_point_view_destroy(m_rustView);
        m_rustView = nullptr;
    }
    m_rustDims.clear();

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

void FbiWriter::write(const PointViewPtr view)
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

void FbiWriter::done(PointTableRef)
{
    pdal_options_t* options = pdal_options_create();
    addOption(options, "filename", filename());

    pdal_writer_t* writer = pdal_writer_create_fbi(options);
    if (!writer)
    {
        pdal_options_destroy(options);
        throwLastRustError("Failed to create Rust FBI writer.");
    }

    bool ok = pdal_writer_write_view(writer, m_rustView);
    pdal_writer_destroy(writer);
    pdal_options_destroy(options);
    if (!ok)
        throwLastRustError("Rust FBI writer failed.");

    pdal_point_view_destroy(m_rustView);
    m_rustView = nullptr;
}

} // namespace pdal
