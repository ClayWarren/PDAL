/******************************************************************************
 * Copyright (c) 2011, Michael P. Gerlek (mpg@flaxen.com)
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

#include "ReprojectionFilter.hpp"

#include <pdal/PointView.hpp>
#include <pdal/private/SrsTransform.hpp>
#include <pdal/util/ProgramArgs.hpp>
#include <pdal_capi.h>

#include "private/RustViewConverter.hpp"

namespace pdal
{

static StaticPluginInfo const s_info{
    "filters.reprojection",
    "Reproject data using GDAL from one coordinate system to another.",
    "https://pdal.org/stages/filters.reprojection.html"};

CREATE_STATIC_STAGE(ReprojectionFilter, s_info)

std::string ReprojectionFilter::getName() const
{
    return s_info.name;
}

ReprojectionFilter::ReprojectionFilter() : m_inferInputSRS(true) {}

ReprojectionFilter::~ReprojectionFilter()
{
    if (m_rust_stage)
        pdal_stage_destroy(m_rust_stage);
}

void ReprojectionFilter::addArgs(ProgramArgs& args)
{
    args.add("out_srs", "Output spatial reference", m_outSRS).setPositional();
    args.add("in_srs", "Input spatial reference", m_inSRS);
    args.add("in_axis_ordering", "Axis ordering override for in_srs",
             m_inAxisOrderingArg, {});
    args.add("out_axis_ordering", "Axis ordering override for out_srs",
             m_outAxisOrderingArg, {});
    args.add("in_coord_epoch", "Input coordinate epoch for transformation",
             m_inCoordEpochArg);
    args.add("out_coord_epoch", "Output coordinate epoch for transformation",
             m_outCoordEpochArg);
    args.add("error_on_failure",
             "Throw an exception if we can't reproject any point",
             m_errorOnFailure);
}

void ReprojectionFilter::initialize()
{
    m_inferInputSRS = m_inSRS.empty();
    setSpatialReference(m_outSRS);

    if (m_rust_stage)
        pdal_stage_destroy(m_rust_stage);

    m_rust_stage = pdal_stage_create_reprojection(
        m_outSRS.getWKT().c_str(),
        m_inSRS.empty() ? nullptr : m_inSRS.getWKT().c_str(),
        m_errorOnFailure);
}

void ReprojectionFilter::ready(PointTableRef table)
{
    if (m_rust_stage)
        pdal_stage_reset(m_rust_stage);
}

void ReprojectionFilter::spatialReferenceChanged(const SpatialReference& srs)
{
}

void ReprojectionFilter::prepared(PointTableRef table)
{
}

void ReprojectionFilter::createTransform(const SpatialReference& srsSRS)
{
}

PointViewSet ReprojectionFilter::run(PointViewPtr view)
{
    PointViewSet viewSet;
    if (m_rust_stage)
    {
        pdal_point_view_set_spatial_reference((pdal_point_view_t*)view.get(), view->spatialReference().getWKT().c_str());
        viewSet.insert(rust_view_converter::runSingle(m_rust_stage, view));
    }
    return viewSet;
}

bool ReprojectionFilter::processOne(PointRef& point)
{
    if (m_rust_stage)
    {
        pdal_point_view_set_spatial_reference((pdal_point_view_t*)point.view(), point.view()->spatialReference().getWKT().c_str());
        return pdal_stage_process_one(m_rust_stage, (pdal_point_view_t*)point.view(), point.pointId());
    }
    return false;
}

} // namespace pdal
