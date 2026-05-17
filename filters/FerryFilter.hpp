/******************************************************************************
 * Copyright (c) 2014, Howard Butler <hobu.inc@gmail.com>
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

#pragma once

#include <pdal/Filter.hpp>
#include <pdal/Streamable.hpp>

#include <string>
#include <vector>

struct pdal_stage;

namespace pdal
{

class PDAL_EXPORT FerryFilter : public Filter, public Streamable
{
    struct Info
    {
        std::string m_fromName;
        std::string m_toName;
        Dimension::Id m_fromId;
        Dimension::Id m_toId;

        Info(const std::string& fromName, const std::string& toName)
            : m_fromName(fromName), m_toName(toName),
              m_fromId(Dimension::Id::Unknown), m_toId(Dimension::Id::Unknown)
        {
        }
    };

public:
    FerryFilter();
    ~FerryFilter() override;

    std::string getName() const override;

private:
    void addArgs(ProgramArgs& args) override;
    void initialize() override;
    void addDimensions(PointLayoutPtr layout) override;
    void prepared(PointTableRef table) override;
    bool processOne(PointRef& point) override;
    PointViewSet run(PointViewPtr view) override;
    void filter(PointView& view) override;

    FerryFilter& operator=(const FerryFilter&) = delete;
    FerryFilter(const FerryFilter&) = delete;

    StringList m_dimSpec;
    std::vector<Info> m_dims;
    pdal_stage* m_rust_stage;
};

} // namespace pdal
