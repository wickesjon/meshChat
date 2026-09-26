package org.meshchat.ui

internal data class MeasurementPlan(val mode: String, val initiator: String) {
    init {
        require(mode in listOf("smoke", "acceptance"))
        require(initiator in listOf("A", "B"))
        require(mode != "smoke" || initiator == "A")
    }
    val pairs get() = if (mode == "smoke") 7 else 60
    fun long(index: Int): Boolean {
        require(index in 0 until pairs)
        return if (mode == "smoke") index in 3..5 else index % 2 == 1
    }
    fun background(index: Int) = mode == "smoke" && index == 6
}
