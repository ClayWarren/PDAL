#pragma once

#include <pdal/PointView.hpp>
#include <pdal/pdal_types.hpp>
#include <pdal_capi.h>

namespace pdal
{
namespace rust_view_converter
{

inline pdal_point_view_t* toRust(PointView& inView)
{
    pdal_point_layout_t* layout = pdal_point_layout_create();
    for (auto dim : inView.layout()->dims())
    {
        pdal_point_layout_register_dim(
            layout, inView.layout()->dimName(dim).c_str(), 9);
    }
    pdal_point_view_t* rust_in_view = pdal_point_view_create(layout);
    for (PointId idx = 0; idx < inView.size(); ++idx)
    {
        pdal_point_view_add_point(rust_in_view);
        for (auto dim : inView.layout()->dims())
        {
            double v = inView.getFieldAs<double>(dim, idx);
            pdal_point_view_set_f64(rust_in_view, idx,
                                    inView.layout()->dimName(dim).c_str(), v);
        }
    }
    // Ownership of layout is transferred to rust_in_view. Do not destroy layout
    // here.
    return rust_in_view;
}

inline pdal_point_view_t* toRust(PointViewPtr inView)
{
    return toRust(*inView);
}

inline void fromRust(pdal_point_view_t* rust_out_view, PointView& outView)
{
    if (rust_out_view)
    {
        uint64_t out_len = pdal_point_view_length(rust_out_view);
        for (PointId idx = 0; idx < out_len; ++idx)
        {
            PointId out_idx = idx;
            if (out_idx >= outView.size())
            {
                outView.point(out_idx);
            }
            for (auto dim : outView.layout()->dims())
            {
                double v = pdal_point_view_get_f64(
                    rust_out_view, idx, outView.layout()->dimName(dim).c_str());
                outView.setField(dim, out_idx, v);
            }
        }
    }
}

inline PointViewPtr fromRust(pdal_point_view_t* rust_out_view,
                             PointViewPtr baseView)
{
    PointViewPtr outView = baseView->makeNew();
    fromRust(rust_out_view, *outView);
    return outView;
}

inline PointViewPtr runSingle(pdal_stage_t* stage, PointViewPtr inView)
{
    pdal_point_view_t* rustIn = toRust(inView);
    pdal_point_view_t* rustOut = pdal_stage_run(stage, rustIn);
    pdal_point_view_destroy(rustIn);

    if (!rustOut)
    {
        const char* message = pdal_last_error();
        if (message && message[0])
            throw pdal_error(message);
        return inView->makeNew();
    }

    PointViewPtr outView = fromRust(rustOut, inView);
    pdal_point_view_destroy(rustOut);
    return outView;
}

} // namespace rust_view_converter
} // namespace pdal
