#pragma once

#include "Expression.hpp"

namespace pdal
{
namespace expr
{

class MathExpression : public Expression
{
public:
    Utils::StatusWithReason prepare(PointLayoutPtr layout) override;
    double eval(PointRef& p) const;
};

} // namespace expr
} // namespace pdal
