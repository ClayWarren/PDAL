/******************************************************************************
 * Copyright (c) 2016, Hobu Inc. (info@hobu.co)
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
 *     * Neither the name of Hobu, Inc. names of its contributors may be
 *       used to endorse or promote products derived from this software
 *       without specific prior written permission.
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

#include <pdal/Scaling.hpp>
#include <pdal_capi.h>

#include <vector>

namespace pdal
{

namespace
{

pdal_xform_component_t toRustComponent(const XForm::XFormComponent& component)
{
    return {component.m_auto, component.m_val};
}

pdal_xform_t toRustXForm(const XForm& xform)
{
    return {toRustComponent(xform.m_offset), toRustComponent(xform.m_scale)};
}

void fromRustComponent(const pdal_xform_component_t& rustComponent,
                       XForm::XFormComponent& component)
{
    component.m_auto = rustComponent.is_auto;
    component.m_val = rustComponent.value;
}

void fromRustXForm(const pdal_xform_t& rustXForm, XForm& xform)
{
    fromRustComponent(rustXForm.offset, xform.m_offset);
    fromRustComponent(rustXForm.scale, xform.m_scale);
}

} // unnamed namespace

void Scaling::addArgs(ProgramArgs& args)
{
    m_xOffArg = &args.add("offset_x", "X offset", m_xXform.m_offset);
    m_yOffArg = &args.add("offset_y", "Y offset", m_yXform.m_offset);
    m_zOffArg = &args.add("offset_z", "Z offset", m_zXform.m_offset);

    m_xScaleArg = &args.add("scale_x", "X scale", m_xXform.m_scale,
                            XForm::XFormComponent(1.0));
    m_yScaleArg = &args.add("scale_y", "Y scale", m_yXform.m_scale,
                            XForm::XFormComponent(1.0));
    m_zScaleArg = &args.add("scale_z", "Z scale", m_zXform.m_scale,
                            XForm::XFormComponent(1.0));
}

void Scaling::setAutoXForm(const PointViewSet& pvSet)
{
    bool xmod = m_xXform.m_offset.m_auto || m_xXform.m_scale.m_auto;
    bool ymod = m_yXform.m_offset.m_auto || m_yXform.m_scale.m_auto;
    bool zmod = m_zXform.m_offset.m_auto || m_zXform.m_scale.m_auto;
    if (!xmod && !ymod && !zmod)
        return;

    std::vector<double> xs;
    std::vector<double> ys;
    std::vector<double> zs;

    for (const PointViewPtr& view : pvSet)
    {
        for (PointId idx = 0; idx < view->size(); idx++)
        {
            xs.push_back(view->getFieldAs<double>(Dimension::Id::X, idx));
            ys.push_back(view->getFieldAs<double>(Dimension::Id::Y, idx));
            zs.push_back(view->getFieldAs<double>(Dimension::Id::Z, idx));
        }
    }

    pdal_scaling_t rustScaling = {toRustXForm(m_xXform), toRustXForm(m_yXform),
                                  toRustXForm(m_zXform)};
    if (pdal_scaling_set_auto_xform(xs.data(), ys.data(), zs.data(), xs.size(),
                                    &rustScaling))
    {
        fromRustXForm(rustScaling.x, m_xXform);
        fromRustXForm(rustScaling.y, m_yXform);
        fromRustXForm(rustScaling.z, m_zXform);
    }
}

} // namespace pdal
