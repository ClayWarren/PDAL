/******************************************************************************
 * Copyright (c) 2019, Aurelien Vila (aurelien.vila@delair.aero)
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

#include <memory>

struct pdal_stage;
typedef struct pdal_stage pdal_stage_t;

class OGRCoordinateTransformation;

namespace pdal
{

class PDAL_EXPORT ProjPipelineFilter : public Filter, public Streamable
{
public:
    class CoordTransform;

    ProjPipelineFilter();
    ~ProjPipelineFilter() override;

    std::string getName() const override;

private:
    ProjPipelineFilter& operator=(const ProjPipelineFilter&) = delete;
    ProjPipelineFilter(const ProjPipelineFilter&) = delete;

    void addArgs(ProgramArgs& args) override;
    void initialize() override;
    void ready(PointTableRef table) override;
    PointViewSet run(PointViewPtr view) override;
    bool processOne(PointRef& point) override;

    void createTransform(const std::string coordOperation, bool reverseTransfo);

    SpatialReference m_outSRS;
    bool m_reverseTransfo;
    std::string m_coordOperation;
    std::unique_ptr<CoordTransform> m_coordTransform;

    // Rust stage instance
    pdal_stage_t* m_rust_stage = nullptr;
};

class ProjPipelineFilter::CoordTransform
{
public:
    CoordTransform();
    CoordTransform(const std::string coordOperation, bool reverseTransfo);

    bool transform(double& x, double& y, double& z);

private:
    std::unique_ptr<OGRCoordinateTransformation> m_transform;
};

} // namespace pdal
